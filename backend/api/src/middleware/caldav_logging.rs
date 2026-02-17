use axum::{
    body::{Body, Bytes},
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;

// 1MB Limit for debug logging to prevent memory exhaustion
const MAX_DEBUG_BODY_SIZE: usize = 1024 * 1024;

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

    // Capture request body ONLY if debug is enabled
    // This prevents unbounded memory consumption in production
    let req = if debug_enabled {
        let (parts, body) = req.into_parts();
        match buffer_and_log_body(body, "Request Body").await {
            Ok(bytes) => Request::from_parts(parts, Body::from(bytes)),
            Err(response) => return response,
        }
    } else {
        req
    };

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
        // For response body, we swallow errors and return empty body to avoid crashing the client response
        // though strictly we should probably log the error and return the partial/empty body.
        // Returning a 413 for a *response* doesn't make sense here.
        let bytes = match buffer_and_log_body(body, "Response Body").await {
            Ok(b) => b,
            Err(response) => {
                let status = response.status();
                tracing::warn!(
                    status = %status,
                    "Failed to buffer response body for logging"
                );
                Bytes::new()
            },
        };

        return Response::from_parts(parts, Body::from(bytes));
    }

    response
}

async fn buffer_and_log_body(body: Body, label: &str) -> Result<Bytes, Response> {
    // Enforce size limit to prevent DoS
    match axum::body::to_bytes(body, MAX_DEBUG_BODY_SIZE).await {
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
            Ok(bytes)
        }
        Err(e) => {
            tracing::error!("Failed to read {} (limit exceeded?): {}", label, e);
            // If this is a request body (implied by context where we propagate error), return 413
            // Note: axum::body::to_bytes returns Error if limit exceeded.
            // We assume mostly limit errors here or IO errors.
            Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                format!(
                    "Debug log body limit exceeded (max {} bytes)",
                    MAX_DEBUG_BODY_SIZE
                ),
            )
                .into_response())
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
