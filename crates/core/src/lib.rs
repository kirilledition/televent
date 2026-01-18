//! Televent Core - Domain logic and models
//!
//! This crate contains pure domain logic with no I/O operations.
//! All database models, business logic, and error types are defined here.

pub mod error;
pub mod models;

pub use error::CalendarError;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
