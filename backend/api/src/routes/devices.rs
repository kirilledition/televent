//! Device password management endpoints
//!
//! Provides REST API for managing CalDAV device passwords

use axum::{
    Extension, Json, Router,
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use televent_core::models::UserId;
use typeshare::typeshare;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};

// Input validation constants
const MAX_DEVICE_NAME_LENGTH: usize = 128;
const MIN_DEVICE_NAME_LENGTH: usize = 1;
// Limit device count to prevent DoS via CPU exhaustion during CalDAV auth
const MAX_DEVICES_PER_USER: i64 = 10;

/// Request to create a new device password
#[typeshare]
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeviceRequest {
    /// Device name/label (e.g., "iPhone", "Desktop")
    #[schema(example = "iPhone")]
    pub name: String,
}

impl CreateDeviceRequest {
    /// Validate the request fields
    pub fn validate(&self) -> Result<(), ApiError> {
        let name_len = self.name.trim().len();
        if name_len < MIN_DEVICE_NAME_LENGTH {
            return Err(ApiError::BadRequest(
                "Device name cannot be empty".to_string(),
            ));
        }
        if name_len > MAX_DEVICE_NAME_LENGTH {
            return Err(ApiError::BadRequest(format!(
                "Device name too long (max {} characters)",
                MAX_DEVICE_NAME_LENGTH
            )));
        }
        Ok(())
    }
}

/// Response containing generated device password
#[typeshare]
#[derive(Debug, Serialize, ToSchema)]
pub struct DevicePasswordResponse {
    #[typeshare(serialized_as = "string")]
    pub id: Uuid,
    pub name: String,
    /// Plain text password - only shown once at creation
    #[schema(example = "aB1c2D3e4F5g6H7i8J9k0L1m")]
    pub password: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Device password list item (without password)
#[typeshare]
#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceListItem {
    #[typeshare(serialized_as = "string")]
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Device password database row
#[derive(Debug, sqlx::FromRow)]
struct DevicePassword {
    id: Uuid,
    #[allow(dead_code)]
    user_id: UserId,
    #[allow(dead_code)]
    password_hash: String,
    device_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Create a new device password
#[utoipa::path(
    post,
    path = "/devices",
    request_body = CreateDeviceRequest,
    responses(
        (status = 201, description = "Device password created", body = DevicePasswordResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "devices",
    security(
        ("telegram_auth" = [])
    )
)]
async fn create_device_password(
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Json(request): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate input
    request.validate()?;

    // Check device limit
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM device_passwords WHERE user_id = $1")
        .bind(auth_user.id)
        .fetch_one(&pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {e}")))?;

    if count >= MAX_DEVICES_PER_USER {
        return Err(ApiError::BadRequest(format!(
            "Maximum number of devices ({}) reached. Please delete an old device password.",
            MAX_DEVICES_PER_USER
        )));
    }

    // Generate random password
    let password = generate_password(24);

    // Hash with Argon2id (blocking task)
    let password_clone = password.clone();
    let hashed = tokio::task::spawn_blocking(move || {
        use argon2::password_hash::rand_core::OsRng;
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let argon2 = argon2::Argon2::default();
        argon2::PasswordHasher::hash_password(&argon2, password_clone.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Task join error: {}", e)))?
    .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?;

    // Insert into database
    let device = sqlx::query_as::<_, DevicePassword>(
        r#"
        INSERT INTO device_passwords (user_id, device_name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(auth_user.id)
    .bind(&request.name)
    .bind(&hashed)
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(DevicePasswordResponse {
            id: device.id,
            name: device.device_name,
            password: Some(password), // Only shown at creation
            created_at: device.created_at.to_rfc3339(),
            last_used_at: device.last_used_at.map(|t| t.to_rfc3339()),
        }),
    ))
}

/// List all device passwords for a user
#[utoipa::path(
    get,
    path = "/devices",
    responses(
        (status = 200, description = "List of device passwords", body = Vec<DeviceListItem>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "devices",
    security(
        ("telegram_auth" = [])
    )
)]
async fn list_device_passwords(
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
) -> Result<Json<Vec<DeviceListItem>>, ApiError> {
    let devices = sqlx::query_as::<_, DevicePassword>(
        "SELECT * FROM device_passwords WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth_user.id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<DeviceListItem> = devices
        .into_iter()
        .map(|d| DeviceListItem {
            id: d.id,
            name: d.device_name,
            created_at: d.created_at.to_rfc3339(),
            last_used_at: d.last_used_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

/// Delete a device password
#[utoipa::path(
    delete,
    path = "/devices/{device_id}",
    responses(
        (status = 204, description = "Device password deleted successfully"),
        (status = 404, description = "Device password not found"),
        (status = 401, description = "Unauthorized")
    ),
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    tag = "devices",
    security(
        ("telegram_auth" = [])
    )
)]
async fn delete_device_password(
    State(pool): State<PgPool>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Path(device_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let result = sqlx::query("DELETE FROM device_passwords WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(auth_user.id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Device password not found: {}",
            device_id
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Generate a random alphanumeric password
fn generate_password(length: usize) -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Device password routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    PgPool: FromRef<S>,
{
    Router::new()
        .route("/devices", post(create_device_password))
        .route("/devices", get(list_device_passwords))
        .route("/devices/{device_id}", delete(delete_device_password))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_password_length() {
        let pwd = generate_password(24);
        assert_eq!(pwd.len(), 24);
    }

    #[test]
    fn test_generate_password_alphanumeric() {
        let pwd = generate_password(100);
        assert!(pwd.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_password_unique() {
        let pwd1 = generate_password(24);
        let pwd2 = generate_password(24);
        assert_ne!(pwd1, pwd2);
    }

    #[test]
    fn test_create_device_request_validation_success() {
        let req = CreateDeviceRequest {
            name: "iPhone".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_device_request_validation_empty_name() {
        let req = CreateDeviceRequest {
            name: "".to_string(),
        };
        assert!(req.validate().is_err());

        // Whitespace-only should also fail
        let req = CreateDeviceRequest {
            name: "   ".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_device_request_validation_too_long() {
        let req = CreateDeviceRequest {
            name: "a".repeat(MAX_DEVICE_NAME_LENGTH + 1),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_device_request_validation_max_length() {
        let req = CreateDeviceRequest {
            name: "a".repeat(MAX_DEVICE_NAME_LENGTH),
        };
        assert!(req.validate().is_ok());
    }
}
