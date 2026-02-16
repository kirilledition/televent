//! CalDAV basic authentication middleware
//!
//! Handles HTTP Basic Auth for CalDAV routes.
//! Authenticates against `device_passwords` table using Argon2.

use crate::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use televent_core::models::UserId;
use tracing::{debug, warn};
use sqlx::Row;

/// Middleware to enforce Basic Auth for CalDAV
pub async fn caldav_basic_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // 1. Get Authorization header
    let auth_header = match req.headers().get(header::AUTHORIZATION) {
        Some(h) => h,
        None => return unauthorized("Missing Authorization header"),
    };

    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return unauthorized("Invalid Authorization header encoding"),
    };

    // 2. Parse "Basic <base64>"
    if !auth_str.starts_with("Basic ") {
        return unauthorized("Only Basic authentication is supported");
    }

    let credentials = match STANDARD.decode(&auth_str[6..]) {
        Ok(c) => c,
        Err(_) => return unauthorized("Invalid Base64 in Authorization header"),
    };

    let credentials_str = match std::str::from_utf8(&credentials) {
        Ok(s) => s,
        Err(_) => return unauthorized("Invalid UTF-8 in credentials"),
    };

    // 3. Split "username:password"
    // Telegram user ID is used as username
    let (username, password) = match credentials_str.split_once(':') {
        Some((u, p)) => (u, p),
        None => return unauthorized("Invalid credential format"),
    };

    let login_id = match username.parse::<i64>() {
        Ok(id) => id,
        Err(_) => return unauthorized("Username must be a numeric Telegram ID"),
    };

    // 4. Check Cache first (fast path)
    let cache_key = format!("{}:{}", login_id, password);

    if state.auth_cache.get(&cache_key).await.is_some() {
        debug!("CalDAV auth cache hit for user {}", login_id);
        let user_id = UserId::new(login_id);

        // Add user_id to request extensions for downstream handlers
        let mut req = req;
        req.extensions_mut().insert(user_id);
        return next.run(req).await;
    }

    // 5. Verify against Database (slow path)
    let user_valid = verify_device_password(&state.pool, login_id, password).await;

    match user_valid {
        Ok(true) => {
            debug!("CalDAV auth success for user {}", login_id);
            // Cache success
            state.auth_cache.insert(cache_key, ()).await;

            let user_id = UserId::new(login_id);
            let mut req = req;
            req.extensions_mut().insert(user_id);
            next.run(req).await
        }
        Ok(false) => {
            warn!("CalDAV auth failed for user {}: invalid password", login_id);
            unauthorized("Invalid username or password")
        }
        Err(e) => {
            warn!("CalDAV auth internal error for user {}: {}", login_id, e);
            unauthorized("Internal authentication error")
        }
    }
}

fn unauthorized(msg: &str) -> Response {
    warn!("CalDAV Unauthorized: {}", msg);
    (
        StatusCode::UNAUTHORIZED,
        [
            (header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic realm=\"Televent CalDAV\"")),
            (header::CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        msg.to_string(),
    )
    .into_response()
}

async fn verify_device_password(
    pool: &sqlx::PgPool,
    user_id: i64,
    password: &str,
) -> anyhow::Result<bool> {
    // Use sqlx::query instead of sqlx::query! macro to avoid compile-time DB checks
    let rows = sqlx::query("SELECT password_hash FROM device_passwords WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    if rows.is_empty() {
        return Ok(false);
    }

    let argon2 = argon2::Argon2::default();

    for row in rows {
        let password_hash: String = row.try_get("password_hash")?;

        let parsed_hash = match argon2::PasswordHash::new(&password_hash) {
            Ok(h) => h,
            Err(e) => {
                warn!("Invalid password hash in DB for user {}: {}", user_id, e);
                continue;
            }
        };

        if argon2::PasswordVerifier::verify_password(&argon2, password.as_bytes(), &parsed_hash).is_ok() {
            // Update last_used_at (fire and forget / async)
            let pool_clone = pool.clone();
            let hash_clone = password_hash.clone();
            tokio::spawn(async move {
                let _ = sqlx::query("UPDATE device_passwords SET last_used_at = NOW() WHERE password_hash = $1")
                    .bind(hash_clone)
                    .execute(&pool_clone)
                    .await;
            });

            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::get, Router};
    use tower::ServiceExt;

    // Helper to create valid basic auth header
    fn basic_auth(user: &str, pass: &str) -> String {
        format!("Basic {}", STANDARD.encode(format!("{}:{}", user, pass)))
    }

    #[tokio::test]
    async fn test_parse_basic_auth_valid_numeric() {
        // Placeholder test
    }
}
