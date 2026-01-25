//! Health check endpoint

use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Server status ("ok" or "degraded")
    #[schema(example = "ok")]
    pub status: String,
    /// Database status ("healthy" or "unhealthy")
    #[schema(example = "healthy")]
    pub database: String,
}

/// Health check endpoint
///
/// Returns 200 OK if the server and database are healthy
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
        (status = 503, description = "Server is degraded", body = HealthResponse)
    ),
    tag = "health"
)]
async fn health_check(State(pool): State<PgPool>) -> Response {
    // Check database connectivity
    let db_status = match sqlx::query("SELECT 1").fetch_one(&pool).await {
        Ok(_) => "healthy",
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            "unhealthy"
        }
    };

    let response = HealthResponse {
        status: if db_status == "healthy" {
            "ok"
        } else {
            "degraded"
        }
        .to_string(),
        database: db_status.to_string(),
    };

    let status_code = if db_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response)).into_response()
}

/// Health check routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PgPool: FromRef<S>,
{
    Router::new().route("/health", get(health_check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            database: "healthy".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("healthy"));
    }

    #[test]
    fn test_health_response_degraded() {
        let response = HealthResponse {
            status: "degraded".to_string(),
            database: "unhealthy".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("degraded"));
        assert!(json.contains("unhealthy"));
    }

    // Note: Integration test for the actual health endpoint requires a database connection
    // and should be in tests/integration_tests.rs
}
