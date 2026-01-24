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

use crate::Config;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

/// Send an email via SMTP
///
/// Uses configuration for SMTP server settings
pub async fn send_email(config: &Config, to: &str, subject: &str, body: &str) -> Result<()> {
    let smtp_host = &config.smtp_host;
    let smtp_port = config.smtp_port;
    let smtp_from = &config.smtp_from;

    // Build email message
    let email = Message::builder()
        .from(
            smtp_from
                .parse()
                .map_err(|e| MailerError::InvalidAddress(format!("Invalid from address: {}", e)))?,
        )
        .to(to
            .parse()
            .map_err(|e| MailerError::InvalidAddress(format!("Invalid to address: {}", e)))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| MailerError::SendFailed(format!("Failed to build message: {}", e)))?;

    // Configure SMTP transport
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        if let (Some(username), Some(password)) = (&config.smtp_username, &config.smtp_password) {
            // Authenticated SMTP
            AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
                .map_err(|e| {
                    MailerError::ConnectionFailed(format!("Failed to create transport: {}", e))
                })?
                .port(smtp_port)
                .credentials(Credentials::new(username.clone(), password.clone()))
                .build()
        } else {
            // Unauthenticated SMTP (for local testing)
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
                .port(smtp_port)
                .build()
        };

    // Send email
    mailer
        .send(email)
        .await
        .map_err(|e| MailerError::SendFailed(format!("Failed to send email: {}", e)))?;

    tracing::info!("Email sent successfully to {}", to);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use televent_core::config::CoreConfig;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn create_test_config(port: u16) -> Config {
        Config {
            core: CoreConfig {
                database_url: "postgres://localhost".to_string(),
                telegram_bot_token: "test_token".to_string(),
            },
            poll_interval_secs: 10,
            max_retry_count: 5,
            batch_size: 10,
            status_log_interval_secs: 60,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: port,
            smtp_username: None,
            smtp_password: None,
            smtp_from: "test@televent.app".to_string(),
        }
    }

    #[test]
    fn test_mailer_error_types() {
        let err = MailerError::SendFailed("test".to_string());
        assert!(err.to_string().contains("Failed to send email"));

        let err = MailerError::InvalidAddress("test".to_string());
        assert!(err.to_string().contains("Invalid email address"));

        let err = MailerError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("SMTP connection failed"));
    }

    #[tokio::test]
    async fn test_send_email_success() {
        // Find a random free port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Spawn a mock SMTP server
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();

            // SMTP Handshake
            socket.write_all(b"220 localhost ESMTP\r\n").await.unwrap();

            let mut buf = [0; 1024];

            // EHLO
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.starts_with("EHLO"), "Expected EHLO, got {}", req);
            socket
                .write_all(b"250-localhost\r\n250 8BITMIME\r\n")
                .await
                .unwrap();

            // MAIL FROM
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("MAIL FROM:<test@televent.app>"));
            socket.write_all(b"250 2.1.0 Ok\r\n").await.unwrap();

            // RCPT TO
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("RCPT TO:<recipient@example.com>"));
            socket.write_all(b"250 2.1.5 Ok\r\n").await.unwrap();

            // DATA
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("DATA"));
            socket
                .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                .await
                .unwrap();

            // Content - read until we get the terminator
            let mut email_data = String::new();
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                let chunk = String::from_utf8_lossy(&buf[..n]);
                email_data.push_str(&chunk);
                if email_data.contains("\r\n.\r\n") {
                    break;
                }
            }

            assert!(email_data.contains("Subject: Test Subject"));
            assert!(email_data.contains("Test Body"));
            socket.write_all(b"250 2.0.0 Ok: queued\r\n").await.unwrap();

            // QUIT
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("QUIT"));
            socket.write_all(b"221 2.0.0 Bye\r\n").await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send email
        let config = create_test_config(port);
        let result = send_email(
            &config,
            "recipient@example.com",
            "Test Subject",
            "Test Body",
        )
        .await;

        assert!(result.is_ok(), "Failed to send email: {:?}", result.err());

        server.await.unwrap();
    }
}
