use sqlx::PgPool;

use crate::StorageResult;

#[derive(Clone)]
pub struct HealthRepository {
    pool: PgPool,
}

impl HealthRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn check_database(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}
