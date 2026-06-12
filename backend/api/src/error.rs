//! Error handling for API endpoints

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use televent_application::ApplicationError;

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// API error type that can be converted to HTTP responses
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden,
    Conflict(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, details) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", Some(msg)),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", Some(msg)),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", Some(msg)),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", None),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", Some(msg)),
            ApiError::Internal(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    None,
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            details,
        });

        // Add WWW-Authenticate header for 401 Unauthorized responses
        // Required by RFC 2617 for HTTP Basic Auth
        // Include helpful hint about using Telegram ID as username
        // We DO NOT add WWW-Authenticate header here by default because:
        // 1. It triggers native browser login prompt which confuses Telegram Mini App users
        // 2. CalDAV auth is handled separately by `caldav_basic_auth`
        // 3. Telegram authentication is custom header-based

        // if status == StatusCode::UNAUTHORIZED { ... }

        (status, body).into_response()
    }
}

impl From<ApplicationError> for ApiError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::NotFound(msg) => ApiError::NotFound(msg),
            ApplicationError::BadRequest(msg) => ApiError::BadRequest(msg),
            ApplicationError::Conflict(msg) => ApiError::Conflict(msg),
            ApplicationError::Internal(msg) => ApiError::Internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse {
            error: "Not Found".to_string(),
            details: Some("Resource does not exist".to_string()),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Not Found"));
        assert!(json.contains("Resource does not exist"));
    }

    #[test]
    fn test_error_response_without_details() {
        let error = ErrorResponse {
            error: "Forbidden".to_string(),
            details: None,
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Forbidden"));
        assert!(!json.contains("details"));
    }
}
