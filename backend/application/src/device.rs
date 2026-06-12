use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chrono::{DateTime, Utc};
use rand::RngExt;
use televent_storage::device::{DevicePasswordHash, DeviceRepository, StoredDevicePassword};
use uuid::Uuid;

use crate::{ApplicationError, UserId, storage_error};

pub const PASSWORD_LEN: usize = 24;
const MAX_DEVICE_NAME_LENGTH: usize = 128;
const MIN_DEVICE_NAME_LENGTH: usize = 1;
const MAX_DEVICES_PER_USER: i64 = 10;

#[derive(Clone)]
pub struct DeviceService {
    devices: DeviceRepository,
}

impl DeviceService {
    #[must_use]
    pub fn new(devices: DeviceRepository) -> Self {
        Self { devices }
    }

    pub async fn create_device_password(
        &self,
        command: CreateDevicePasswordCommand,
    ) -> Result<CreatedDevicePassword, ApplicationError> {
        validate_device_name(&command.name)?;

        let password = generate_password(PASSWORD_LEN);
        let password_hash = hash_password(password.clone()).await?;

        let mut tx = self.devices.begin().await.map_err(storage_error)?;
        tx.ensure_user(command.user_id.inner(), command.username.as_deref())
            .await
            .map_err(storage_error)?;

        let user_id = command.user_id;
        let count = tx
            .count_device_passwords(user_id)
            .await
            .map_err(storage_error)?;
        if count >= MAX_DEVICES_PER_USER {
            return Err(ApplicationError::BadRequest(format!(
                "Maximum number of devices ({MAX_DEVICES_PER_USER}) reached. Please delete an old device password."
            )));
        }

        let device = tx
            .insert_device_password(StoredDevicePassword {
                user_id,
                name: command.name.trim().to_string(),
                password_hash,
            })
            .await
            .map_err(storage_error)?;

        tx.commit().await.map_err(storage_error)?;

        Ok(CreatedDevicePassword {
            id: device.id,
            name: device.name,
            password,
            created_at: device.created_at,
            last_used_at: device.last_used_at,
        })
    }

    pub async fn list_device_passwords(
        &self,
        user_id: UserId,
    ) -> Result<Vec<DevicePasswordView>, ApplicationError> {
        let devices = self
            .devices
            .list_device_passwords(user_id)
            .await
            .map_err(storage_error)?;
        Ok(devices
            .into_iter()
            .map(|device| DevicePasswordView {
                id: device.id,
                name: device.name,
                created_at: device.created_at,
                last_used_at: device.last_used_at,
            })
            .collect())
    }

    pub async fn revoke_device_password(
        &self,
        user_id: UserId,
        device_id: Uuid,
    ) -> Result<bool, ApplicationError> {
        self.devices
            .delete_device_password(user_id, device_id)
            .await
            .map_err(storage_error)
    }

    pub async fn list_password_hashes_for_auth(
        &self,
        user_id: UserId,
    ) -> Result<Vec<DevicePasswordHash>, ApplicationError> {
        self.devices
            .list_device_password_hashes(user_id, MAX_DEVICES_PER_USER)
            .await
            .map_err(storage_error)
    }

    pub async fn record_device_used(&self, device_id: Uuid) -> Result<(), ApplicationError> {
        self.devices
            .touch_device_password(device_id)
            .await
            .map_err(storage_error)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreateDevicePasswordCommand {
    pub user_id: UserId,
    pub username: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CreatedDevicePassword {
    pub id: Uuid,
    pub name: String,
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DevicePasswordView {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub fn validate_device_name(name: &str) -> Result<(), ApplicationError> {
    let name_len = name.trim().len();
    if name_len < MIN_DEVICE_NAME_LENGTH {
        return Err(ApplicationError::BadRequest(
            "Device name cannot be empty".to_string(),
        ));
    }
    if name_len > MAX_DEVICE_NAME_LENGTH {
        return Err(ApplicationError::BadRequest(format!(
            "Device name too long (max {MAX_DEVICE_NAME_LENGTH} characters)"
        )));
    }
    Ok(())
}

fn generate_password(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

async fn hash_password(password: String) -> Result<String, ApplicationError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|err| ApplicationError::Internal(format!("Password hashing failed: {err}")))
    })
    .await
    .map_err(|err| ApplicationError::Internal(format!("Task join error: {err}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_password_has_expected_length() {
        assert_eq!(generate_password(PASSWORD_LEN).len(), PASSWORD_LEN);
    }

    #[test]
    fn generated_password_is_alphanumeric() {
        assert!(
            generate_password(100)
                .chars()
                .all(|char| char.is_ascii_alphanumeric())
        );
    }

    #[test]
    fn validates_device_name() {
        assert!(validate_device_name("iPhone").is_ok());
        assert!(validate_device_name("").is_err());
        assert!(validate_device_name("   ").is_err());
        assert!(validate_device_name(&"x".repeat(MAX_DEVICE_NAME_LENGTH + 1)).is_err());
    }
}
