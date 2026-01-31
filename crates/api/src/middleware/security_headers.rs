//! Security headers middleware
//!
//! Adds standard security headers to all responses to improve security.
//!
//! Headers added:
//! - X-Content-Type-Options: nosniff
//! - X-XSS-Protection: 1; mode=block
//! - Strict-Transport-Security: max-age=31536000; includeSubDomains
//! - Referrer-Policy: strict-origin-when-cross-origin

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

/// Middleware to add security headers
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // Enable XSS protection in older browsers
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Enforce HTTPS (HSTS) - 1 year
    // Note: This is ignored by browsers on HTTP connections, but useful for the initial redirect
    // or if behind a TLS-terminating proxy that doesn't add it.
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Control Referrer header
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy
    // - script-src/style-src: unsafe-inline required for Next.js static export + Swagger UI
    // - frame-ancestors: restricted to Telegram domains for Mini App support
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors https://web.telegram.org https://*.telegram.org; img-src 'self' data: https:; connect-src 'self';"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::StatusCode, routing::get};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_security_headers() {
        let app = Router::new()
            .route("/", get(|| async { "Hello" }))
            .layer(axum::middleware::from_fn(security_headers));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();

        assert_eq!(
            headers.get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get("X-XSS-Protection").unwrap(),
            "1; mode=block"
        );
        assert_eq!(
            headers.get("Strict-Transport-Security").unwrap(),
            "max-age=31536000; includeSubDomains"
        );
        assert_eq!(
            headers.get("Referrer-Policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            headers.get("Content-Security-Policy").unwrap(),
            "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors https://web.telegram.org https://*.telegram.org; img-src 'self' data: https:; connect-src 'self';"
        );
    }
}
