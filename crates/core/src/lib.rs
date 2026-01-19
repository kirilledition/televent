//! Televent Core - Domain logic and models
//!
//! This crate contains pure domain logic with no I/O operations.
//! All database models, business logic, and error types are defined here.

pub mod error;
pub mod models;
pub mod timezone;

pub use error::CalendarError;
pub use timezone::{parse_timezone, to_timezone, to_utc, validate_timezone};
