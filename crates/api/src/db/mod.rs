//! Database repository modules

pub mod calendars;
pub mod events;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use chrono::{Utc, TimeZone};
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_deleted_events_sync_flow(pool: PgPool) {
        // Run migrations
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate");

        // 1. Create a User
        let telegram_id = 123456789i64;
        let user_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (telegram_id, telegram_username) VALUES ($1, $2) RETURNING id"
        )
        .bind(telegram_id)
        .bind("testuser")
        .fetch_one(&pool)
        .await
        .expect("Failed to create user");

        // 2. Create Calendar
        let calendar = calendars::get_or_create_calendar(&pool, user_id)
            .await
            .expect("Failed to create calendar");

        assert_eq!(calendar.sync_token, "0");

        // 3. Create Event
        let uid = "test-event-uid";
        let now = Utc::now();
        let start = Utc.timestamp_opt(now.timestamp() + 3600, 0).unwrap();
        let end = Utc.timestamp_opt(now.timestamp() + 7200, 0).unwrap();

        events::create_event(
            &pool,
            calendar.id,
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

        // 4. Simulate Deletion Flow (Transaction: Increment -> Delete)
        // This replicates the logic in caldav_delete_event
        let mut tx = pool.begin().await.expect("Failed to start transaction");

        // Increment sync token (should become "1")
        let new_token_str = calendars::increment_sync_token_tx(&mut tx, calendar.id)
            .await
            .expect("Failed to increment sync token");

        assert_eq!(new_token_str, "1");

        // Delete event
        let deleted = events::delete_event_by_uid_tx(&mut tx, calendar.id, uid)
            .await
            .expect("Failed to delete event");

        assert!(deleted, "Event should have been deleted");

        tx.commit().await.expect("Failed to commit transaction");

        // 5. Verify deleted_events has the correct token
        // We expect the trigger to have captured "1" (the new token)
        let (deletion_token,): (i64,) = sqlx::query_as(
            "SELECT deletion_token FROM deleted_events WHERE calendar_id = $1 AND uid = $2"
        )
        .bind(calendar.id)
        .bind(uid)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch deleted event");

        assert_eq!(deletion_token, 1, "Deletion token should be 1");

        // 6. Verify list_deleted_events_since_sync behavior

        // Case A: Client syncs from 0 (requesting changes > 0)
        // Should return the deletion (because 1 > 0)
        let deleted_since_0 = events::list_deleted_events_since_sync(&pool, calendar.id, 0)
            .await
            .expect("Failed to list deleted events");
        assert!(deleted_since_0.contains(&uid.to_string()), "Should contain deleted event when syncing from 0");

        // Case B: Client syncs from 1 (requesting changes > 1)
        // Should NOT return the deletion (because 1 > 1 is False)
        // This implies the client already has state 1, which includes this deletion.
        let deleted_since_1 = events::list_deleted_events_since_sync(&pool, calendar.id, 1)
            .await
            .expect("Failed to list deleted events");
        assert!(!deleted_since_1.contains(&uid.to_string()), "Should NOT contain deleted event when syncing from 1");
    }
}
