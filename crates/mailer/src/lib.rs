//! Email sending functionality for Televent
//!
//! This crate provides email sending capabilities using the Lettre library.

use anyhow::Result;
use tracing::{error, info};

/// Email configuration
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from_address: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "mailpit".to_string(),
            smtp_port: 1025,
            from_address: "noreply@televent.app".to_string(),
        }
    }
}

/// Email sender
pub struct EmailSender {
    config: EmailConfig,
}

impl EmailSender {
    /// Create a new email sender with the given configuration
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    /// Send an email
    ///
    /// # Errors
    ///
    /// Returns an error if the email fails to send
    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        info!(to = %to, subject = %subject, "Sending email");

        // TODO: Implement actual email sending with lettre
        // - Create Message with from/to/subject/body
        // - Create SmtpTransport with relay and port
        // - Send the email
        // - Handle errors appropriately

        info!(to = %to, "Email sent successfully (stub implementation)");
        Ok(())
    }

    /// Send a reminder email for an event
    pub async fn send_event_reminder(
        &self,
        to: &str,
        event_title: &str,
        event_start: &str,
    ) -> Result<()> {
        let subject = format!("Reminder: {}", event_title);
        let body = format!(
            "This is a reminder for your event:\n\n{}\n\nStarts at: {}",
            event_title, event_start
        );
        self.send_email(to, &subject, &body).await
    }

    /// Send a daily digest email
    pub async fn send_daily_digest(
        &self,
        to: &str,
        events: &[(String, String)], // (title, time)
    ) -> Result<()> {
        let subject = "Your daily event digest".to_string();
        let body = if events.is_empty() {
            "You have no events scheduled for today.".to_string()
        } else {
            let mut body = "Here are your events for today:\n\n".to_string();
            for (title, time) in events {
                body.push_str(&format!("• {} at {}\n", title, time));
            }
            body
        };
        self.send_email(to, &subject, &body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_default() {
        let config = EmailConfig::default();
        assert_eq!(config.smtp_host, "mailpit");
        assert_eq!(config.smtp_port, 1025);
    }

    #[tokio::test]
    async fn test_email_sender_creation() {
        let config = EmailConfig::default();
        let sender = EmailSender::new(config);
        // Stub test - actual implementation would test email sending
        assert!(sender.send_email("test@example.com", "Test", "Body").await.is_ok());
    }
}
