use axum::{
    body::{Body, Bytes},
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

/// Middleware to log deep details about CalDAV requests
pub async fn caldav_logger(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    // Check if we should log verbose details (DEBUG level)
    let debug_enabled = std::env::var("CALDAV_DEBUG").is_ok();

    // Log request start
    tracing::info!(
        method = %method,
        path = %uri,
        "CalDAV request started"
    );

    if debug_enabled {
        log_headers(&headers, "Request");
    }

    // Capture request body if debug is enabled
    let (parts, body) = req.into_parts();
    let bytes = if debug_enabled {
        buffer_and_log_body(body, "Request Body").await
    } else {
        match axum::body::to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to read request body: {}", e);
                Bytes::new()
            }
        }
    };

    let req = Request::from_parts(parts, Body::from(bytes));

    // Execute handler
    let response = next.run(req).await;

    // Log response
    let status = response.status();
    let duration = start.elapsed();

    tracing::info!(
        method = %method,
        path = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "CalDAV request completed"
    );

    if debug_enabled {
        // We need to double-buffer response body to log it
        let (parts, body) = response.into_parts();
        let bytes = buffer_and_log_body(body, "Response Body").await;

        return Response::from_parts(parts, Body::from(bytes));
    }

    response
}

async fn buffer_and_log_body(body: Body, label: &str) -> Bytes {
    match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            if !bytes.is_empty() {
                if let Ok(body_str) = std::str::from_utf8(&bytes) {
                    tracing::debug!("{}:\n{}", label, body_str);
                } else {
                    tracing::debug!("{}: <binary data {} bytes>", label, bytes.len());
                }
            } else {
                tracing::debug!("{}: <empty>", label);
            }
            bytes
        }
        Err(e) => {
            tracing::error!("Failed to read {}: {}", label, e);
            Bytes::new()
        }
    }
}

fn log_headers(headers: &HeaderMap, label: &str) {
    tracing::debug!("{} Headers:", label);
    for (name, value) in headers {
        if name == axum::http::header::AUTHORIZATION {
            tracing::debug!("  {}: <REDACTED>", name);
        } else {
            tracing::debug!("  {}: {:?}", name, value);
        }
    }
}
