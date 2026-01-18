//! Event repository for database operations

use crate::error::ApiError;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use televent_core::models::{Event, EventStatus};
use uuid::Uuid;

/// Create a new event
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
    timezone: String,
    rrule: Option<String>,
) -> Result<Event, ApiError> {
    // Validate time range
    if end <= start {
        return Err(ApiError::BadRequest(
            "Event end time must be after start time".to_string(),
        ));
    }

    // Generate ETag (SHA256 of event data)
    let etag = generate_etag(&uid, &summary, &start, &end);

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
    .bind(description)
    .bind(location)
    .bind(start)
    .bind(end)
    .bind(is_all_day)
    .bind(EventStatus::Confirmed)
    .bind(&timezone)
    .bind(rrule)
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

/// List events for a calendar within a time range
pub async fn list_events(
    pool: &PgPool,
    calendar_id: Uuid,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<Event>, ApiError> {
    let events = match (start, end) {
        (Some(start_time), Some(end_time)) => {
            sqlx::query_as::<_, Event>(
                r#"
                SELECT * FROM events
                WHERE calendar_id = $1
                AND start >= $2
                AND start < $3
                ORDER BY start ASC
                "#,
            )
            .bind(calendar_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, Event>(
                "SELECT * FROM events WHERE calendar_id = $1 ORDER BY start ASC",
            )
            .bind(calendar_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(events)
}

/// Update an existing event
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
    // Get current event to check version
    let current = get_event(pool, event_id).await?;

    // Build update query dynamically based on provided fields
    let mut query = String::from("UPDATE events SET version = version + 1");
    let mut params: Vec<String> = vec![];
    let mut param_count = 1;

    if let Some(ref s) = summary {
        query.push_str(&format!(", summary = ${}", param_count));
        params.push(s.clone());
        param_count += 1;
    }

    if description.is_some() {
        query.push_str(&format!(", description = ${}", param_count));
        params.push(description.clone().unwrap_or_default());
        param_count += 1;
    }

    if location.is_some() {
        query.push_str(&format!(", location = ${}", param_count));
        params.push(location.clone().unwrap_or_default());
        param_count += 1;
    }

    if let Some(s) = start {
        query.push_str(&format!(", start = ${}", param_count));
        params.push(s.to_rfc3339());
        param_count += 1;
    }

    if let Some(e) = end {
        query.push_str(&format!(r#", "end" = ${}"#, param_count));
        params.push(e.to_rfc3339());
        param_count += 1;
    }

    if let Some(a) = is_all_day {
        query.push_str(&format!(", is_all_day = ${}", param_count));
        params.push(a.to_string());
        param_count += 1;
    }

    if let Some(ref st) = status {
        query.push_str(&format!(", status = ${}", param_count));
        params.push(format!("{:?}", st).to_uppercase());
        param_count += 1;
    }

    if rrule.is_some() {
        query.push_str(&format!(", rrule = ${}", param_count));
        params.push(rrule.clone().unwrap_or_default());
        param_count += 1;
    }

    // Generate new ETag
    let new_summary = summary.clone().unwrap_or(current.summary);
    let new_start = start.unwrap_or(current.start);
    let new_end = end.unwrap_or(current.end);
    let new_etag = generate_etag(&current.uid, &new_summary, &new_start, &new_end);

    query.push_str(&format!(", etag = ${}", param_count));
    query.push_str(&format!(", updated_at = NOW() WHERE id = ${}", param_count + 1));
    query.push_str(" RETURNING *");

    // For simplicity, we'll use a simpler approach with explicit fields
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
        return Err(ApiError::NotFound(format!(
            "Event not found: {}",
            event_id
        )));
    }

    Ok(())
}

/// Generate ETag for an event (SHA256 hash)
fn generate_etag(
    uid: &str,
    summary: &str,
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
) -> String {
    let data = format!("{}|{}|{}|{}", uid, summary, start.to_rfc3339(), end.to_rfc3339());
    let hash = Sha256::digest(data.as_bytes());
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

        let etag1 = generate_etag(uid, summary, &start, &end);
        let etag2 = generate_etag(uid, summary, &start, &end);

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

        let etag1 = generate_etag(uid, summary1, &start, &end);
        let etag2 = generate_etag(uid, summary2, &start, &end);

        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_generate_etag_changes_with_time() {
        let uid = "test-uid-123";
        let summary = "Test Event";
        let start1 = "2026-01-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let start2 = "2026-01-18T11:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let end = "2026-01-18T12:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let etag1 = generate_etag(uid, summary, &start1, &end);
        let etag2 = generate_etag(uid, summary, &start2, &end);

        assert_ne!(etag1, etag2);
    }
}
