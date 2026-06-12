//! SQLx storage layer for Televent.
//!
//! Storage owns table shape and SQL. Application services own transaction
//! boundaries and calendar mutation invariants.

pub mod calendar;
pub mod device;
pub mod health;
pub mod outbox;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid database data: {0}")]
    InvalidData(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
