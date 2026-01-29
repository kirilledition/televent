//! Rate limiting middleware
//!
//! Implements rate limiting using `tower-governor`.

use axum::{extract::ConnectInfo, http::Request};
use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};
use tower_governor::{errors::GovernorError, key_extractor::KeyExtractor};
use uuid::Uuid;

// Target rates:
// - CalDAV: 100 requests/minute = 1 request every 600ms
pub const CALDAV_PERIOD_MS: u64 = 600;
pub const CALDAV_BURST_SIZE: u32 = 100;

// - API: 300 requests/minute = 1 request every 200ms
pub const API_PERIOD_MS: u64 = 200;
pub const API_BURST_SIZE: u32 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    User(Uuid),
    Ip(IpAddr),
}

#[derive(Clone)]
pub struct UserOrIpKeyExtractor;

impl KeyExtractor for UserOrIpKeyExtractor {
    type Key = RateLimitKey;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, GovernorError> {
        if let Some(user_id) = req.extensions().get::<Uuid>() {
            return Ok(RateLimitKey::User(*user_id));
        }

        let headers = req.headers();

        // 1. Try X-Forwarded-For (standard for proxies like Nginx/Railway)
        if let Some(header) = headers.get("x-forwarded-for") {
            if let Ok(val) = header.to_str() {
                // Takes the first IP in the list (Client, Proxy1, Proxy2)
                if let Some(client_ip) = val.split(',').next() {
                    if let Ok(ip) = client_ip.trim().parse::<IpAddr>() {
                        return Ok(RateLimitKey::Ip(ip));
                    }
                }
            }
        }

        // 2. Try X-Real-IP
        if let Some(header) = headers.get("x-real-ip") {
            if let Ok(val) = header.to_str() {
                if let Ok(ip) = val.trim().parse::<IpAddr>() {
                    return Ok(RateLimitKey::Ip(ip));
                }
            }
        }

        // 3. Fallback to direct connection IP
        if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            return Ok(RateLimitKey::Ip(addr.ip()));
        }

        Err(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use std::convert::Infallible;
    use std::time::Duration;
    use tower::{Service, ServiceBuilder, ServiceExt};
    use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

    #[tokio::test]
    async fn test_rate_limit_key_extraction() {
        let extractor = UserOrIpKeyExtractor;

        // Test User ID extraction
        let user_id = Uuid::new_v4();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(user_id);
        let key = extractor.extract(&req).unwrap();
        assert_eq!(key, RateLimitKey::User(user_id));

        // Test IP extraction
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));
        let key = extractor.extract(&req).unwrap();
        assert_eq!(key, RateLimitKey::Ip(addr.ip()));
    }

    #[tokio::test]
    async fn test_rate_limit_key_extraction_with_headers() {
        let extractor = UserOrIpKeyExtractor;

        // Test X-Forwarded-For extraction
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));
        req.headers_mut().insert("x-forwarded-for", "203.0.113.195".parse().unwrap());

        let key = extractor.extract(&req).unwrap();

        // It should respect the header
        assert_eq!(key, RateLimitKey::Ip("203.0.113.195".parse().unwrap()));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        // Create a rate limiter with small quota for testing
        // 2 requests per burst, replenish slowly
        let config = GovernorConfigBuilder::default()
            .period(Duration::from_secs(1))
            .burst_size(2)
            .key_extractor(UserOrIpKeyExtractor)
            .finish()
            .unwrap();

        let mut service = ServiceBuilder::new()
            .layer(GovernorLayer::new(config))
            .service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(axum::response::Response::new(Body::empty()))
            });

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 1st request - OK
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));
        let res = service.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), 200);

        // 2nd request - OK
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));
        let res = service.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), 200);

        // 3rd request - Too Many Requests (burst exceeded)
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));

        // Note: With NoOpMiddleware (default), it might return an error that we need to handle.
        // But we want to ensure it works.
        match service.ready().await.unwrap().call(req).await {
            Ok(res) => {
                // If it returns a response, it should be 429
                assert_eq!(res.status(), 429);
            }
            Err(e) => {
                panic!("Expected 429 response, got error: {:?}", e);
            }
        }
    }
}
