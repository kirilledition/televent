//! Event repository for database operations

use crate::error::ApiError;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use televent_core::models::{Event, EventStatus};
use televent_core::timezone::Timezone;
use uuid::Uuid;

/// Create a new event
#[allow(clippy::too_many_arguments)]
pub async fn create_event(
    pool: &PgPool,
    calendar_id: Uuid,
    uid: String,
    summary: String,
    description: Option<String>,
    location: Option<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    is_all_day: bool,
    timezone: Timezone,
    rrule: Option<String>,
) -> Result<Event, ApiError> {
    // Validate time range
    if end <= start {
        return Err(ApiError::BadRequest(
            "Event end time must be after start time".to_string(),
        ));
    }

    let status = EventStatus::Confirmed;

    // Generate ETag (SHA256 of event data)
    let etag = generate_etag(
        &uid,
        &summary,
        description.as_deref(),
        location.as_deref(),
        &start,
        &end,
        is_all_day,
        &status,
        rrule.as_deref(),
    );

    let event = sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (
            calendar_id, uid, summary, description, location,
            start, "end", is_all_day, status, timezone, rrule, etag
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(calendar_id)
    .bind(&uid)
    .bind(&summary)
    .bind(&description)
    .bind(&location)
    .bind(start)
    .bind(end)
    .bind(is_all_day)
    .bind(status)
    .bind(&timezone)
    .bind(&rrule)
    .bind(&etag)
    .fetch_one(pool)
    .await?;

    Ok(event)
}

/// Get event by ID
pub async fn get_event(pool: &PgPool, event_id: Uuid) -> Result<Event, ApiError> {
    let event = sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = $1")
        .bind(event_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Event not found: {}", event_id)))?;

    Ok(event)
}

/// Get event by UID and calendar ID
pub async fn get_event_by_uid(
    pool: &PgPool,
    calendar_id: Uuid,
    uid: &str,
) -> Result<Option<Event>, ApiError> {
    let event =
        sqlx::query_as::<_, Event>("SELECT * FROM events WHERE calendar_id = $1 AND uid = $2")
            .bind(calendar_id)
            .bind(uid)
            .fetch_optional(pool)
            .await?;

    Ok(event)
}

/// Get multiple events by UIDs and calendar ID
pub async fn get_events_by_uids(
    pool: &PgPool,
    calendar_id: Uuid,
    uids: &[&str],
) -> Result<Vec<Event>, ApiError> {
    let events =
        sqlx::query_as::<_, Event>("SELECT * FROM events WHERE calendar_id = $1 AND uid = ANY($2)")
            .bind(calendar_id)
            .bind(uids)
            .fetch_all(pool)
            .await?;

    Ok(events)
}

/// Delete event by UID
#[allow(dead_code)]
pub async fn delete_event_by_uid(
    pool: &PgPool,
    calendar_id: Uuid,
    uid: &str,
) -> Result<bool, ApiError> {
    let result = sqlx::query("DELETE FROM events WHERE calendar_id = $1 AND uid = $2")
        .bind(calendar_id)
        .bind(uid)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete event by UID (within transaction)
pub async fn delete_event_by_uid_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    calendar_id: Uuid,
    uid: &str,
) -> Result<bool, ApiError> {
    let result = sqlx::query("DELETE FROM events WHERE calendar_id = $1 AND uid = $2")
        .bind(calendar_id)
        .bind(uid)
        .execute(&mut **tx)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// List events for a calendar within a time range
pub async fn list_events(
    pool: &PgPool,
    calendar_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Event>, ApiError> {
    let offset = offset.unwrap_or(0);

    let events = match (start, end) {
        (Some(start_time), Some(end_time)) => {
            sqlx::query_as::<_, Event>(
                r#"
                SELECT * FROM events
                WHERE calendar_id = $1
                AND start >= $2
                AND start < $3
                ORDER BY start ASC
                LIMIT $4 OFFSET $5
                "#,
            )
            .bind(calendar_id)
            .bind(start_time)
            .bind(end_time)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, Event>(
                r#"
                SELECT * FROM events
                WHERE calendar_id = $1
                ORDER BY start ASC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(calendar_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(events)
}

/// List UIDs of events deleted since a specific time
///
/// Note: sync_token is treated as a timestamp in some contexts, but here we assume it maps roughly
/// to versioning. For deletions, we need to map the sync_token to a timestamp or use the token directly
/// if we store deletion tokens.
/// For this implementation, we'll assume the sync_token implies we want deletions that happened
/// "recently". However, since our sync_token is just an integer counter, mapping it to deleted_events
/// (which has a timestamp) is tricky without a "deletion version".
///
/// List UIDs of events deleted since a specific time
///
/// Uses deletion_token in deleted_events to filter efficiently.
pub async fn list_deleted_events_since_sync(
    pool: &PgPool,
    calendar_id: Uuid,
    sync_token: i64,
) -> Result<Vec<String>, ApiError> {
    let uids = sqlx::query_scalar::<_, String>(
        "SELECT uid FROM deleted_events WHERE calendar_id = $1 AND deletion_token > $2",
    )
    .bind(calendar_id)
    .bind(sync_token)
    .fetch_all(pool)
    .await?;

    Ok(uids)
}

/// List events modified since a specific sync token
///
/// Used for CalDAV sync-collection REPORT
pub async fn list_events_since_sync(
    pool: &PgPool,
    calendar_id: Uuid,
    sync_token: i64,
) -> Result<Vec<Event>, ApiError> {
    // We use the calendar's sync_token as a version number
    // Events with version > sync_token have been modified since
    let events = sqlx::query_as::<_, Event>(
        r#"
        SELECT * FROM events
        WHERE calendar_id = $1
        AND version > $2
        ORDER BY updated_at ASC
        "#,
    )
    .bind(calendar_id)
    .bind(sync_token as i32)
    .fetch_all(pool)
    .await?;

    Ok(events)
}

/// Update an existing event
#[allow(clippy::too_many_arguments)]
pub async fn update_event(
    pool: &PgPool,
    event_id: Uuid,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    is_all_day: Option<bool>,
    status: Option<EventStatus>,
    rrule: Option<String>,
) -> Result<Event, ApiError> {
    // Get current event to compute new ETag with merged fields
    let current = get_event(pool, event_id).await?;

    // Compute new field values for ETag generation
    let new_summary = summary.clone().unwrap_or_else(|| current.summary.clone());
    let new_description = description.clone().or_else(|| current.description.clone());
    let new_location = location.clone().or_else(|| current.location.clone());
    let new_start = start.unwrap_or(current.start);
    let new_end = end.unwrap_or(current.end);
    let new_is_all_day = is_all_day.unwrap_or(current.is_all_day);
    let new_status = status.unwrap_or(current.status);
    let new_rrule = if rrule.is_some() {
        rrule.clone()
    } else {
        current.rrule.clone()
    };

    // Generate new ETag with all fields
    let new_etag = generate_etag(
        &current.uid,
        &new_summary,
        new_description.as_deref(),
        new_location.as_deref(),
        &new_start,
        &new_end,
        new_is_all_day,
        &new_status,
        new_rrule.as_deref(),
    );

    let event = sqlx::query_as::<_, Event>(
        r#"
        UPDATE events
        SET summary = COALESCE($2, summary),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            start = COALESCE($5, start),
            "end" = COALESCE($6, "end"),
            is_all_day = COALESCE($7, is_all_day),
            status = COALESCE($8, status),
            rrule = COALESCE($9, rrule),
            version = version + 1,
            etag = $10,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(event_id)
    .bind(summary)
    .bind(description)
    .bind(location)
    .bind(start)
    .bind(end)
    .bind(is_all_day)
    .bind(status)
    .bind(rrule)
    .bind(new_etag)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Event not found: {}", event_id)))?;

    Ok(event)
}

/// Delete an event
pub async fn delete_event(pool: &PgPool, event_id: Uuid) -> Result<(), ApiError> {
    let result = sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(event_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Event not found: {}", event_id)));
    }

    Ok(())
}

/// Generate ETag for an event (SHA256 hash)
///
/// Includes all mutable fields to ensure ETag changes when any field changes
#[allow(clippy::too_many_arguments)]
fn generate_etag(
    uid: &str,
    summary: &str,
    description: Option<&str>,
    location: Option<&str>,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    is_all_day: bool,
    status: &EventStatus,
    rrule: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(uid);
    hasher.update("|");
    hasher.update(summary);
    hasher.update("|");
    hasher.update(description.unwrap_or(""));
    hasher.update("|");
    hasher.update(location.unwrap_or(""));
    hasher.update("|");
    // to_rfc3339 allocates, but it's small compared to the full string
    hasher.update(start.to_rfc3339());
    hasher.update("|");
    hasher.update(end.to_rfc3339());
    hasher.update("|");
    hasher.update(if is_all_day { "true" } else { "false" });
    hasher.update("|");
    match status {
        EventStatus::Confirmed => hasher.update("Confirmed"),
        EventStatus::Tentative => hasher.update("Tentative"),
        EventStatus::Cancelled => hasher.update("Cancelled"),
    }
    hasher.update("|");
    hasher.update(rrule.unwrap_or(""));

    let hash = hasher.finalize();
    format!("{:x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_etag_consistent() {
        let uid = "test-uid-123";
        let summary = "Test Event";
        let start = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let status = EventStatus::Confirmed;

        let etag1 = generate_etag(uid, summary, None, None, &start, &end, false, &status, None);
        let etag2 = generate_etag(uid, summary, None, None, &start, &end, false, &status, None);

        assert_eq!(etag1, etag2);
        assert_eq!(etag1.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_generate_etag_different_for_different_data() {
        let uid = "test-uid-123";
        let summary1 = "Test Event 1";
        let summary2 = "Test Event 2";
        let start = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let status = EventStatus::Confirmed;

        let etag1 = generate_etag(
            uid, summary1, None, None, &start, &end, false, &status, None,
        );
        let etag2 = generate_etag(
            uid, summary2, None, None, &start, &end, false, &status, None,
        );

        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_generate_etag_changes_with_time() {
        let uid = "test-uid-123";
        let summary = "Test Event";
        let start1 = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let start2 = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let status = EventStatus::Confirmed;

        let etag1 = generate_etag(
            uid, summary, None, None, &start1, &end, false, &status, None,
        );
        let etag2 = generate_etag(
            uid, summary, None, None, &start2, &end, false, &status, None,
        );

        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_generate_etag_changes_with_description() {
        let uid = "test-uid-123";
        let summary = "Test Event";
        let start = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let status = EventStatus::Confirmed;

        let etag1 = generate_etag(uid, summary, None, None, &start, &end, false, &status, None);
        let etag2 = generate_etag(
            uid,
            summary,
            Some("Description"),
            None,
            &start,
            &end,
            false,
            &status,
            None,
        );

        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_generate_etag_changes_with_status() {
        let uid = "test-uid-123";
        let summary = "Test Event";
        let start = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let etag1 = generate_etag(
            uid,
            summary,
            None,
            None,
            &start,
            &end,
            false,
            &EventStatus::Confirmed,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary,
            None,
            None,
            &start,
            &end,
            false,
            &EventStatus::Cancelled,
            None,
        );

        assert_ne!(etag1, etag2);
    }
}
