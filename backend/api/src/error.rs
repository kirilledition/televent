//! Error handling for API endpoints

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use televent_core::CalendarError;
use utoipa::ToSchema;

/// API error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// API error type that can be converted to HTTP responses
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden")]
    Forbidden,
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal Server Error: {0}")]
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
