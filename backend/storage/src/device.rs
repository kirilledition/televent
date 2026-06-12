use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use televent_domain::UserId;
use uuid::Uuid;

use crate::StorageResult;
use crate::calendar::User;

#[derive(Clone)]
pub struct DeviceRepository {
    pool: PgPool,
}

impl DeviceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn begin(&self) -> StorageResult<DeviceTransaction<'_>> {
        let tx = self.pool.begin().await?;
        Ok(DeviceTransaction { tx })
    }

    pub async fn list_device_passwords(
        &self,
        user_id: UserId,
    ) -> StorageResult<Vec<DevicePasswordRecord>> {
        list_device_passwords(&self.pool, user_id).await
    }

    pub async fn list_device_password_hashes(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> StorageResult<Vec<DevicePasswordHash>> {
        list_device_password_hashes(&self.pool, user_id, limit).await
    }

    pub async fn delete_device_password(
        &self,
        user_id: UserId,
        device_id: Uuid,
    ) -> StorageResult<bool> {
        delete_device_password(&self.pool, user_id, device_id).await
    }

    pub async fn touch_device_password(&self, device_id: Uuid) -> StorageResult<()> {
        touch_device_password(&self.pool, device_id).await
    }
}

pub struct DeviceTransaction<'a> {
    tx: Transaction<'a, Postgres>,
}

impl DeviceTransaction<'_> {
    pub async fn ensure_user(
        &mut self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> StorageResult<User> {
        crate::calendar::ensure_user_tx(&mut self.tx, telegram_id, username).await
    }

    pub async fn count_device_passwords(&mut self, user_id: UserId) -> StorageResult<i64> {
        self::count_device_passwords_tx(&mut self.tx, user_id).await
    }

    pub async fn insert_device_password(
        &mut self,
        password: StoredDevicePassword,
    ) -> StorageResult<DevicePasswordRecord> {
        self::insert_device_password_tx(&mut self.tx, password).await
    }

    pub async fn commit(self) -> StorageResult<()> {
        self.tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DevicePasswordRecord {
    pub id: Uuid,
    pub user_id: i64,
    pub password_hash: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DevicePasswordHash {
    pub id: Uuid,
    pub password_hash: String,
}

pub struct StoredDevicePassword {
    pub user_id: UserId,
    pub name: String,
    pub password_hash: String,
}

async fn count_device_passwords_tx(conn: &mut PgConnection, user_id: UserId) -> StorageResult<i64> {
    let count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM device_passwords WHERE user_id = $1")
            .bind(user_id.inner())
            .fetch_one(conn)
            .await?;

    Ok(count)
}

async fn insert_device_password_tx(
    conn: &mut PgConnection,
    password: StoredDevicePassword,
) -> StorageResult<DevicePasswordRecord> {
    let device = sqlx::query_as::<_, DevicePasswordRecord>(
        r#"
        INSERT INTO device_passwords (user_id, device_name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, password_hash, device_name AS name, created_at, last_used_at
        "#,
    )
    .bind(password.user_id.inner())
    .bind(password.name)
    .bind(password.password_hash)
    .fetch_one(conn)
    .await?;

    Ok(device)
}

async fn list_device_passwords(
    pool: &PgPool,
    user_id: UserId,
) -> StorageResult<Vec<DevicePasswordRecord>> {
    let devices = sqlx::query_as::<_, DevicePasswordRecord>(
        r#"
        SELECT id, user_id, password_hash, device_name AS name, created_at, last_used_at
        FROM device_passwords
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id.inner())
    .fetch_all(pool)
    .await?;

    Ok(devices)
}

async fn list_device_password_hashes(
    pool: &PgPool,
    user_id: UserId,
    limit: i64,
) -> StorageResult<Vec<DevicePasswordHash>> {
    let devices = sqlx::query_as::<_, DevicePasswordHash>(
        r#"
        SELECT id, password_hash
        FROM device_passwords
        WHERE user_id = $1
        ORDER BY last_used_at DESC NULLS LAST, created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id.inner())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(devices)
}

async fn delete_device_password(
    pool: &PgPool,
    user_id: UserId,
    device_id: Uuid,
) -> StorageResult<bool> {
    let result = sqlx::query("DELETE FROM device_passwords WHERE id = $1 AND user_id = $2")
        .bind(device_id)
        .bind(user_id.inner())
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

async fn touch_device_password(pool: &PgPool, device_id: Uuid) -> StorageResult<()> {
    sqlx::query("UPDATE device_passwords SET last_used_at = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(pool)
        .await?;

    Ok(())
}
