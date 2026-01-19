//! Device password management endpoints
//!
//! Provides REST API for managing CalDAV device passwords

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

/// Request to create a new device password
#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
}

/// Response containing generated device password
#[derive(Debug, Serialize)]
pub struct DevicePasswordResponse {
    pub id: Uuid,
    pub name: String,
    /// Plain text password - only shown once at creation
    pub password: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Device password list item (without password)
#[derive(Debug, Serialize)]
pub struct DeviceListItem {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Device password database row
#[derive(Debug, sqlx::FromRow)]
struct DevicePassword {
    id: Uuid,
    user_id: Uuid,
    hashed_password: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Create a new device password
async fn create_device_password(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Generate random password
    let password = generate_password(24);

    // Hash with Argon2id
    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = argon2::Argon2::default();
    let hashed = argon2::PasswordHasher::hash_password(&argon2, password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(format!("Password hashing failed: {}", e)))?
        .to_string();

    // Insert into database
    let device = sqlx::query_as::<_, DevicePassword>(
        r#"
        INSERT INTO device_passwords (user_id, name, hashed_password)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&request.name)
    .bind(&hashed)
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(DevicePasswordResponse {
            id: device.id,
            name: device.name,
            password: Some(password), // Only shown at creation
            created_at: device.created_at.to_rfc3339(),
            last_used_at: device.last_used_at.map(|t| t.to_rfc3339()),
        }),
    ))
}

/// List all device passwords for a user
async fn list_device_passwords(
    State(pool): State<PgPool>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<DeviceListItem>>, ApiError> {
    let devices = sqlx::query_as::<_, DevicePassword>(
        "SELECT * FROM device_passwords WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<DeviceListItem> = devices
        .into_iter()
        .map(|d| DeviceListItem {
            id: d.id,
            name: d.name,
            created_at: d.created_at.to_rfc3339(),
            last_used_at: d.last_used_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

/// Delete a device password
async fn delete_device_password(
    State(pool): State<PgPool>,
    Path((user_id, device_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let result = sqlx::query(
        "DELETE FROM device_passwords WHERE id = $1 AND user_id = $2",
    )
    .bind(device_id)
    .bind(user_id)
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
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Device password routes
pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/users/:user_id/devices", post(create_device_password))
        .route("/users/:user_id/devices", get(list_device_passwords))
        .route(
            "/users/:user_id/devices/:device_id",
            delete(delete_device_password),
        )
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
}
