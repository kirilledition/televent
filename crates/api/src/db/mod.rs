//! Database repository modules

pub mod events;
pub mod users;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use sqlx::PgPool;
    use televent_core::models::UserId;

    #[sqlx::test]
    async fn test_user_creation_and_lookup(pool: PgPool) {
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate");

        let telegram_id = 987654321i64;
        let username = "testuser123";

        // Create user
        let user = users::get_or_create_user(&pool, telegram_id, Some(username))
            .await
            .expect("Failed to create user");

        assert_eq!(user.id, UserId::new(telegram_id));
        assert_eq!(user.telegram_username.as_deref(), Some(username));

        // Lookup by ID
        let found = users::get_user_by_id(&pool, user.id)
            .await
            .expect("Failed to get user")
            .expect("User should exist");

        assert_eq!(found.id, user.id);

        // Lookup by username
        let found_by_name = users::get_user_by_username(&pool, username)
            .await
            .expect("Failed to get user")
            .expect("User should exist");

        assert_eq!(found_by_name.id, user.id);
    }
}
