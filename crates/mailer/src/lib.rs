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

use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};

/// Send an email via SMTP
///
/// Uses environment variables for SMTP configuration:
/// - SMTP_HOST: SMTP server hostname
/// - SMTP_PORT: SMTP server port (default: 587)
/// - SMTP_USERNAME: SMTP username (optional)
/// - SMTP_PASSWORD: SMTP password (optional)
/// - SMTP_FROM: From email address
pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<()> {
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "1025".to_string())
        .parse()
        .unwrap_or(1025);
    let smtp_from = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@televent.app".to_string());

    // Build email message
    let email = Message::builder()
        .from(smtp_from.parse().map_err(|e| MailerError::InvalidAddress(format!("Invalid from address: {}", e)))?)
        .to(to.parse().map_err(|e| MailerError::InvalidAddress(format!("Invalid to address: {}", e)))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| MailerError::SendFailed(format!("Failed to build message: {}", e)))?;

    // Configure SMTP transport
    let mailer = if let (Ok(username), Ok(password)) = (
        std::env::var("SMTP_USERNAME"),
        std::env::var("SMTP_PASSWORD"),
    ) {
        // Authenticated SMTP
        SmtpTransport::relay(&smtp_host)
            .map_err(|e| MailerError::ConnectionFailed(format!("Failed to create transport: {}", e)))?
            .port(smtp_port)
            .credentials(Credentials::new(username, password))
            .build()
    } else {
        // Unauthenticated SMTP (for local testing)
        SmtpTransport::builder_dangerous(&smtp_host)
            .port(smtp_port)
            .build()
    };

    // Send email
    mailer
        .send(&email)
        .map_err(|e| MailerError::SendFailed(format!("Failed to send email: {}", e)))?;

    tracing::info!("Email sent successfully to {}", to);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailer_error_types() {
        let err = MailerError::SendFailed("test".to_string());
        assert!(err.to_string().contains("Failed to send email"));

        let err = MailerError::InvalidAddress("test".to_string());
        assert!(err.to_string().contains("Invalid email address"));

        let err = MailerError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("SMTP connection failed"));
    }
}
