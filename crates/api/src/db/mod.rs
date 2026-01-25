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
    async fn test_deleted_events_sync_flow(pool: PgPool) {
        // Run migrations
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate");

        // 1. Create a User (telegram_id is now the primary key)
        let telegram_id = 123456789i64;
        let user = users::get_or_create_user(&pool, telegram_id, Some("testuser"))
            .await
            .expect("Failed to create user");

        let user_id = user.id;
        assert_eq!(user.sync_token, "0");

        // 2. Create Event
        let uid = "test-event-uid";
        let now = Utc::now();
        let start = Utc.timestamp_opt(now.timestamp() + 3600, 0).unwrap();
        let end = Utc.timestamp_opt(now.timestamp() + 7200, 0).unwrap();

        events::create_event(
            &pool,
            user_id,
            uid.to_string(),
            "Test Event".to_string(),
            None,
            None,
            start,
            end,
            false,
            "UTC".to_string(),
            None,
        )
        .await
        .expect("Failed to create event");

        // 3. Simulate Deletion Flow (Transaction: Increment -> Delete)
        let mut tx = pool.begin().await.expect("Failed to start transaction");

        // Increment sync token (should become "1")
        let new_token_str = users::increment_sync_token_tx(&mut tx, user_id)
            .await
            .expect("Failed to increment sync token");

        assert_eq!(new_token_str, "1");

        // Delete event
        let deleted = events::delete_event_by_uid_tx(&mut tx, user_id, uid)
            .await
            .expect("Failed to delete event");

        assert!(deleted, "Event should have been deleted");

        tx.commit().await.expect("Failed to commit transaction");

        // 4. Verify deleted_events has the correct token
        let (deletion_token,): (i64,) = sqlx::query_as(
            "SELECT deletion_token FROM deleted_events WHERE user_id = $1 AND uid = $2",
        )
        .bind(user_id)
        .bind(uid)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch deleted event");

        assert_eq!(deletion_token, 1, "Deletion token should be 1");

        // 5. Verify list_deleted_events_since_sync behavior

        // Case A: Client syncs from 0 (requesting changes > 0)
        let deleted_since_0 = events::list_deleted_events_since_sync(&pool, user_id, 0)
            .await
            .expect("Failed to list deleted events");
        assert!(
            deleted_since_0.contains(&uid.to_string()),
            "Should contain deleted event when syncing from 0"
        );

        // Case B: Client syncs from 1 (requesting changes > 1)
        let deleted_since_1 = events::list_deleted_events_since_sync(&pool, user_id, 1)
            .await
            .expect("Failed to list deleted events");
        assert!(
            !deleted_since_1.contains(&uid.to_string()),
            "Should NOT contain deleted event when syncing from 1"
        );
    }

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
