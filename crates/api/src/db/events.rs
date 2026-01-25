//! Event repository for database operations

use crate::error::ApiError;
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use televent_core::models::{Event, EventStatus, Timezone, UserId};
use uuid::Uuid;

/// Helper Enum to enforce valid input states
pub enum EventTiming {
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    AllDay {
        date: NaiveDate,
        end_date: NaiveDate,
    },
}

/// Create a new event
#[allow(clippy::too_many_arguments)]
pub async fn create_event(
    pool: &PgPool,
    user_id: UserId,
    uid: String,
    summary: String,
    description: Option<String>,
    location: Option<String>,
    timing: EventTiming,
    timezone: Timezone,
    rrule: Option<String>,
) -> Result<Event, ApiError> {
    let (start, end, start_date, end_date, is_all_day) = match timing {
        EventTiming::Timed { start, end } => {
            if end <= start {
                return Err(ApiError::BadRequest(
                    "Event end time must be after start time".to_string(),
                ));
            }
            (Some(start), Some(end), None, None, false)
        }
        EventTiming::AllDay { date, end_date } => {
            if end_date <= date {
                return Err(ApiError::BadRequest(
                    "Event end date must be after start date".to_string(),
                ));
            }
            (None, None, Some(date), Some(end_date), true)
        }
    };

    let status = EventStatus::Confirmed;

    // Generate ETag
    let etag = generate_etag(
        &uid,
        &summary,
        description.as_deref(),
        location.as_deref(),
        start.as_ref(),
        end.as_ref(),
        start_date.as_ref(),
        end_date.as_ref(),
        &status,
        rrule.as_deref(),
    );

    let event = sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (
            user_id, uid, summary, description, location,
            start, "end", start_date, end_date, is_all_day, status, timezone, rrule, etag
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&uid)
    .bind(&summary)
    .bind(&description)
    .bind(&location)
    .bind(start)
    .bind(end)
    .bind(start_date)
    .bind(end_date)
    .bind(is_all_day)
    .bind(status)
    .bind(timezone)
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

/// Get event by UID and user ID
pub async fn get_event_by_uid(
    pool: &PgPool,
    user_id: UserId,
    uid: &str,
) -> Result<Option<Event>, ApiError> {
    let event = sqlx::query_as::<_, Event>("SELECT * FROM events WHERE user_id = $1 AND uid = $2")
        .bind(user_id)
        .bind(uid)
        .fetch_optional(pool)
        .await?;

    Ok(event)
}

/// Get multiple events by UIDs and user ID
pub async fn get_events_by_uids(
    pool: &PgPool,
    user_id: UserId,
    uids: &[&str],
) -> Result<Vec<Event>, ApiError> {
    let events =
        sqlx::query_as::<_, Event>("SELECT * FROM events WHERE user_id = $1 AND uid = ANY($2)")
            .bind(user_id)
            .bind(uids)
            .fetch_all(pool)
            .await?;

    Ok(events)
}

/// Delete event by UID
#[allow(dead_code)]
pub async fn delete_event_by_uid(
    pool: &PgPool,
    user_id: UserId,
    uid: &str,
) -> Result<bool, ApiError> {
    let result = sqlx::query("DELETE FROM events WHERE user_id = $1 AND uid = $2")
        .bind(user_id)
        .bind(uid)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete event by UID (within transaction)
pub async fn delete_event_by_uid_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: UserId,
    uid: &str,
) -> Result<bool, ApiError> {
    let result = sqlx::query("DELETE FROM events WHERE user_id = $1 AND uid = $2")
        .bind(user_id)
        .bind(uid)
        .execute(&mut **tx)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// List events for a user within a time range
pub async fn list_events(
    pool: &PgPool,
    user_id: UserId,
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
                WHERE user_id = $1
                AND start >= $2
                AND start < $3
                ORDER BY start ASC
                LIMIT $4 OFFSET $5
                "#,
            )
            .bind(user_id)
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
                WHERE user_id = $1
                ORDER BY start ASC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(events)
}

/// List events modified since a specific sync token
///
/// Used for CalDAV sync-collection REPORT
pub async fn list_events_since_sync(
    pool: &PgPool,
    user_id: UserId,
    sync_token: i64,
) -> Result<Vec<Event>, ApiError> {
    // We use the user's sync_token as a version number
    // Events with version > sync_token have been modified since
    let events = sqlx::query_as::<_, Event>(
        r#"
        SELECT * FROM events
        WHERE user_id = $1
        AND version > $2
        ORDER BY updated_at ASC
        "#,
    )
    .bind(user_id)
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
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    is_all_day: Option<bool>,
    status: Option<EventStatus>,
    rrule: Option<String>,
) -> Result<Event, ApiError> {
    // Get current event to compute new ETag with merged fields
    let current = get_event(pool, event_id).await?;

    let new_summary = summary.clone().unwrap_or_else(|| current.summary.clone());
    let new_description = description.clone().or_else(|| current.description.clone());
    let new_location = location.clone().or_else(|| current.location.clone());

    let new_is_all_day = is_all_day.unwrap_or(current.is_all_day);

    // Resolve timing fields
    let (new_start, new_end, new_start_date, new_end_date) = if new_is_all_day {
        // Switching to or staying as All Day
        let s_date = start_date.or(current.start_date).ok_or_else(|| {
            ApiError::BadRequest("Missing start_date for all-day event".to_string())
        })?;
        let e_date = end_date.or(current.end_date).ok_or_else(|| {
            ApiError::BadRequest("Missing end_date for all-day event".to_string())
        })?;

        if e_date <= s_date {
            return Err(ApiError::BadRequest(
                "End date must be after start date".to_string(),
            ));
        }
        (None, None, Some(s_date), Some(e_date))
    } else {
        // Switching to or staying as Timed
        let s = start.or(current.start).ok_or_else(|| {
            ApiError::BadRequest("Missing start time for timed event".to_string())
        })?;
        let e = end
            .or(current.end)
            .ok_or_else(|| ApiError::BadRequest("Missing end time for timed event".to_string()))?;

        if e <= s {
            return Err(ApiError::BadRequest(
                "End time must be after start time".to_string(),
            ));
        }
        (Some(s), Some(e), None, None)
    };

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
        new_start.as_ref(),
        new_end.as_ref(),
        new_start_date.as_ref(),
        new_end_date.as_ref(),
        &new_status,
        new_rrule.as_deref(),
    );

    let event = sqlx::query_as::<_, Event>(
        r#"
        UPDATE events
        SET summary = COALESCE($2, summary),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            start = $5,
            "end" = $6,
            start_date = $7,
            end_date = $8,
            is_all_day = $9,
            status = COALESCE($10, status),
            rrule = COALESCE($11, rrule),
            version = version + 1,
            etag = $12,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(event_id)
    .bind(new_summary)
    .bind(new_description)
    .bind(new_location)
    .bind(new_start)
    .bind(new_end)
    .bind(new_start_date)
    .bind(new_end_date)
    .bind(new_is_all_day)
    .bind(new_status)
    .bind(new_rrule)
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
    start: Option<&DateTime<Utc>>,
    end: Option<&DateTime<Utc>>,
    start_date: Option<&NaiveDate>,
    end_date: Option<&NaiveDate>,
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

    if let (Some(s), Some(e)) = (start, end) {
        hasher.update(s.to_rfc3339());
        hasher.update("|");
        hasher.update(e.to_rfc3339());
    } else if let (Some(sd), Some(ed)) = (start_date, end_date) {
        hasher.update(sd.to_string());
        hasher.update("|");
        hasher.update(ed.to_string());
    }

    hasher.update("|");
    // status and rrule...
    match status {
        EventStatus::Confirmed => hasher.update("Confirmed"),
        EventStatus::Tentative => hasher.update("Tentative"),
        EventStatus::Cancelled => hasher.update("Cancelled"),
    }
    hasher.update("|");
    hasher.update(rrule.unwrap_or(""));

    format!("{:x}", hasher.finalize())
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

        let etag1 = generate_etag(
            uid,
            summary,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &status,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &status,
            None,
        );

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
            uid,
            summary1,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &status,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary2,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &status,
            None,
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
            uid,
            summary,
            None,
            None,
            Some(&start1),
            Some(&end),
            None,
            None,
            &status,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary,
            None,
            None,
            Some(&start2),
            Some(&end),
            None,
            None,
            &status,
            None,
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

        let etag1 = generate_etag(
            uid,
            summary,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &status,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary,
            Some("Description"),
            None,
            Some(&start),
            Some(&end),
            None,
            None,
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
            Some(&start),
            Some(&end),
            None,
            None,
            &EventStatus::Confirmed,
            None,
        );
        let etag2 = generate_etag(
            uid,
            summary,
            None,
            None,
            Some(&start),
            Some(&end),
            None,
            None,
            &EventStatus::Cancelled,
            None,
        );

        assert_ne!(etag1, etag2);
    }
}
