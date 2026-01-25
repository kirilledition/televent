//! Error handling for API endpoints

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use televent_core::CalendarError;

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

/// Convert CalendarError to ApiError
impl From<CalendarError> for ApiError {
    fn from(err: CalendarError) -> Self {
        match err {
            CalendarError::EventNotFound(id) => {
                ApiError::NotFound(format!("Event not found: {}", id))
            }
            CalendarError::CalendarNotFound(id) => {
                ApiError::NotFound(format!("Calendar not found: {}", id))
            }
            CalendarError::UserNotFound(id) => {
                ApiError::NotFound(format!("User not found: {}", id))
            }
            CalendarError::VersionConflict { expected, actual } => ApiError::Conflict(format!(
                "Version conflict: expected {}, got {}",
                expected, actual
            )),
            CalendarError::InvalidRRule(msg) => {
                ApiError::BadRequest(format!("Invalid recurrence rule: {}", msg))
            }
            CalendarError::InvalidTimezone(tz) => {
                ApiError::BadRequest(format!("Invalid timezone: {}", tz))
            }
            CalendarError::InvalidEventData(msg) => ApiError::BadRequest(msg),
            CalendarError::PermissionDenied => ApiError::Forbidden,
        }
    }
}

/// Convert sqlx errors to ApiError
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(db_err) => {
                // Check for unique constraint violations
                if let Some(constraint) = db_err.constraint() {
                    ApiError::Conflict(format!("Constraint violation: {}", constraint))
                } else {
                    ApiError::Internal(format!("Database error: {}", db_err))
                }
            }
            _ => ApiError::Internal(format!("Database error: {}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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

    #[test]
    fn test_calendar_error_conversion() {
        let event_id = Uuid::new_v4();
        let err = CalendarError::EventNotFound(event_id);
        let api_err: ApiError = err.into();

        match api_err {
            ApiError::NotFound(msg) => assert!(msg.contains(&event_id.to_string())),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_version_conflict_conversion() {
        let err = CalendarError::VersionConflict {
            expected: 5,
            actual: 3,
        };
        let api_err: ApiError = err.into();

        match api_err {
            ApiError::Conflict(msg) => {
                assert!(msg.contains("expected 5"));
                assert!(msg.contains("got 3"));
            }
            _ => panic!("Expected Conflict error"),
        }
    }

    #[test]
    fn test_permission_denied_conversion() {
        let err = CalendarError::PermissionDenied;
        let api_err: ApiError = err.into();

        match api_err {
            ApiError::Forbidden => {}
            _ => panic!("Expected Forbidden error"),
        }
    }
}
