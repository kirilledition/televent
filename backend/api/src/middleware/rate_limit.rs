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
        // We prioritize this because it's an append-only list where the last entry is reliably added by the proxy.
        // X-Real-IP might be spoofed if the proxy passes it through without overwriting.
        if let Some(header) = headers.get("x-forwarded-for")
            && let Ok(val) = header.to_str()
        {
            // Security: Use the *last* valid IP in the list.
            // X-Forwarded-For appends IPs: "Client, Proxy1, Proxy2".
            // The last IP is the one that connected to the immediate trusted proxy (e.g. Railway LB).
            // Taking the first IP allows spoofing (e.g., "SpoofedIP, RealIP").
            if let Some(ip) = val
                .split(',')
                .rev()
                .find_map(|s| s.trim().parse::<IpAddr>().ok())
            {
                return Ok(RateLimitKey::Ip(ip));
            }
        }

        // 2. Try X-Real-IP (trusted proxy set header)
        // Used as a fallback if X-Forwarded-For is missing or invalid.
        if let Some(header) = headers.get("x-real-ip")
            && let Ok(val) = header.to_str()
            && let Ok(ip) = val.trim().parse::<IpAddr>()
        {
            return Ok(RateLimitKey::Ip(ip));
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
        req.headers_mut()
            .insert("x-forwarded-for", "203.0.113.195".parse().unwrap());

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

    #[tokio::test]
    async fn test_rate_limit_key_extraction_priority() {
        let extractor = UserOrIpKeyExtractor;

        // Test priority: X-Forwarded-For > X-Real-IP
        // We prefer X-Forwarded-For because it's harder to spoof (append-only) behind a proxy
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));

        req.headers_mut()
            .insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        req.headers_mut()
            .insert("x-real-ip", "5.6.7.8".parse().unwrap());

        let key = extractor.extract(&req).unwrap();

        // Should return X-Forwarded-For (1.2.3.4)
        assert_eq!(key, RateLimitKey::Ip("1.2.3.4".parse().unwrap()));
    }
}

#[cfg(test)]
mod spoofing_test {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use tower_governor::key_extractor::KeyExtractor;

    #[tokio::test]
    async fn test_rate_limit_key_extraction_spoofing() {
        let extractor = UserOrIpKeyExtractor;

        // Test X-Forwarded-For spoofing
        // Client sends: X-Forwarded-For: 1.2.3.4 (spoofed)
        // Proxy appends: , 5.6.7.8 (real)
        // Header value: "1.2.3.4, 5.6.7.8"
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));

        req.headers_mut()
            .insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());

        let key = extractor.extract(&req).unwrap();

        // Should return the LAST IP (5.6.7.8), not the first (1.2.3.4)
        assert_eq!(key, RateLimitKey::Ip("5.6.7.8".parse().unwrap()));
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use tower_governor::key_extractor::KeyExtractor;

    #[tokio::test]
    async fn test_x_real_ip_bypass_prevention() {
        let extractor = UserOrIpKeyExtractor;

        // Simulate an attacker sending a spoofed X-Real-IP
        // The trusted proxy (e.g. Railway) appends the real IP to X-Forwarded-For
        // but might pass through the X-Real-IP header if not configured to strip/overwrite it.
        let spoofed_ip: IpAddr = "1.2.3.4".parse().unwrap();
        let real_ip: IpAddr = "5.6.7.8".parse().unwrap();

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let mut req = Request::new(Body::empty());
        req.extensions_mut().insert(ConnectInfo(addr));

        // Attacker sends this:
        req.headers_mut()
            .insert("x-real-ip", spoofed_ip.to_string().parse().unwrap());

        // Proxy appends real IP to this:
        // (Assuming attacker also sent X-Forwarded-For: 1.2.3.4 to try to confuse things)
        req.headers_mut().insert(
            "x-forwarded-for",
            format!("{}, {}", spoofed_ip, real_ip).parse().unwrap(),
        );

        let key = extractor.extract(&req).unwrap();

        // Security check: We must extract the REAL IP (5.6.7.8), not the spoofed one (1.2.3.4)
        // If this assertion fails, it means X-Real-IP took precedence over X-Forwarded-For,
        // allowing the attacker to bypass rate limits by rotating X-Real-IP.
        assert_eq!(
            key,
            RateLimitKey::Ip(real_ip),
            "Vulnerability: X-Real-IP took precedence over X-Forwarded-For"
        );
    }
}
