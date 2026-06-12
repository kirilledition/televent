use televent_storage::health::HealthRepository;

use crate::{ApplicationError, storage_error};

#[derive(Clone)]
pub struct HealthService {
    health: HealthRepository,
}

impl HealthService {
    #[must_use]
    pub fn new(health: HealthRepository) -> Self {
        Self { health }
    }

    pub async fn check_database(&self) -> Result<(), ApplicationError> {
        self.health.check_database().await.map_err(storage_error)?;
        Ok(())
    }
}
