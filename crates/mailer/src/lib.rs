//! Televent Mailer - Email sending service
//!
//! This crate provides email functionality using SMTP.

use thiserror::Error;

/// Mailer errors
#[derive(Error, Debug)]
pub enum MailerError {
    #[error("Failed to send email: {0}")]
    SendFailed(String),
    #[error("Invalid email address: {0}")]
    InvalidAddress(String),
    #[error("SMTP connection failed: {0}")]
    ConnectionFailed(String),
}

/// Result type for mailer operations
pub type Result<T> = std::result::Result<T, MailerError>;
