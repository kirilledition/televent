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
use televent_application::{CreateDevicePasswordCommand, DeviceService, validate_device_name};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::ApiError, middleware::telegram_auth::AuthenticatedTelegramUser};

/// Request to create a new device password
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDeviceRequest {
    /// Device name/label (e.g., "iPhone", "Desktop")
    #[schema(example = "iPhone")]
    pub name: String,
}

impl CreateDeviceRequest {
    /// Validate the request fields
    pub fn validate(&self) -> Result<(), ApiError> {
        validate_device_name(&self.name).map_err(ApiError::from)
    }
}

/// Response containing generated device password
#[derive(Debug, Serialize, ToSchema)]
pub struct DevicePasswordResponse {
    pub id: Uuid,
    pub name: String,
    /// Plain text password - only shown once at creation
    #[schema(example = "aB1c2D3e4F5g6H7i8J9k0L1m")]
    pub password: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Device password list item (without password)
#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceListItem {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
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
    State(device_service): State<DeviceService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Json(request): Json<CreateDeviceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    request.validate()?;
    let device = device_service
        .create_device_password(CreateDevicePasswordCommand {
            user_id: auth_user.id,
            username: None,
            name: request.name,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(DevicePasswordResponse {
            id: device.id,
            name: device.name,
            password: Some(device.password), // Only shown at creation
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
    State(device_service): State<DeviceService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
) -> Result<Json<Vec<DeviceListItem>>, ApiError> {
    let devices = device_service.list_device_passwords(auth_user.id).await?;

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
    State(device_service): State<DeviceService>,
    Extension(auth_user): Extension<AuthenticatedTelegramUser>,
    Path(device_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let revoked = device_service
        .revoke_device_password(auth_user.id, device_id)
        .await?;

    if !revoked {
        return Err(ApiError::NotFound(format!(
            "Device password not found: {}",
            device_id
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Device password routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    DeviceService: FromRef<S>,
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
            name: "a".repeat(129),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_device_request_validation_max_length() {
        let req = CreateDeviceRequest {
            name: "a".repeat(128),
        };
        assert!(req.validate().is_ok());
    }
}
