use axum::{
    extract::Request,
    http::{HeaderValue, Method, header},
    middleware::Next,
    response::Response,
};

/// Middleware to ensure CalDAV headers are present on OPTIONS responses
/// (fixing issue where CorsLayer intercepts OPTIONS and swallows headers)
pub async fn add_caldav_headers(req: Request, next: Next) -> Response {
    let is_options = req.method() == Method::OPTIONS;
    let is_caldav = req.uri().path().starts_with("/caldav");

    let mut response = next.run(req).await;

    if is_caldav && is_options {
        let headers = response.headers_mut();
        // Only add if not present (to allow handler to set them if reached)
        if !headers.contains_key("dav") {
            headers.insert("dav", HeaderValue::from_static("1, calendar-access"));
        }
        if !headers.contains_key(header::ALLOW) {
            headers.insert(
                header::ALLOW,
                HeaderValue::from_static("OPTIONS, PROPFIND, REPORT, GET, PUT, DELETE"),
            );
        }
        if !headers.contains_key("cal-accessible") {
            headers.insert("cal-accessible", HeaderValue::from_static("calendar"));
        }
    }

    response
}
