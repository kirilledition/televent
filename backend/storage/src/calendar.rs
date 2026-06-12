use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder, Row, Transaction};
use std::collections::HashMap;
use televent_domain::{
    AttendeeRole, EventStatus, EventTiming, OutboxPayload, ParticipationStatus, Timezone, UserId,
};
use uuid::Uuid;

use crate::{StorageError, StorageResult};

const USER_COLUMNS: &str =
    "telegram_id, telegram_username, timezone, sync_token, ctag, created_at, updated_at";
const EVENT_COLUMNS: &str = r#"id, user_id, uid, summary, description, location,
    start, "end", start_date, end_date, is_all_day, status::text AS status,
    rrule, timezone, version, sync_version, etag, created_at, updated_at"#;
const ATTENDEE_COLUMNS: &str =
    "event_id, email, user_id, role::text AS role, status::text AS status, created_at, updated_at";

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub telegram_username: Option<String>,
    pub timezone: Timezone,
    pub sync_token: i64,
    pub ctag: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: Uuid,
    pub user_id: UserId,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub timezone: Timezone,
    pub version: i32,
    pub sync_version: i64,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EventAttendee {
    pub event_id: Uuid,
    pub email: String,
    pub user_id: Option<i64>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EventTombstone {
    pub user_id: UserId,
    pub uid: String,
    pub sync_version: i64,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct CalendarRepository {
    pool: PgPool,
}

impl CalendarRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn begin(&self) -> StorageResult<CalendarTransaction<'_>> {
        let tx = self.pool.begin().await?;
        Ok(CalendarTransaction { tx })
    }

    pub async fn ensure_user(
        &self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> StorageResult<User> {
        let mut tx = self.begin().await?;
        let user = tx.ensure_user(telegram_id, username).await?;
        tx.commit().await?;
        Ok(user)
    }

    pub async fn get_event_by_id_any(&self, event_id: Uuid) -> StorageResult<Option<Event>> {
        get_event_by_id_any(&self.pool, event_id).await
    }

    pub async fn get_user_by_id(&self, user_id: UserId) -> StorageResult<Option<User>> {
        get_user_by_id(&self.pool, user_id).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> StorageResult<Option<User>> {
        get_user_by_username(&self.pool, username).await
    }

    pub async fn get_event_by_id(
        &self,
        user_id: UserId,
        event_id: Uuid,
    ) -> StorageResult<Option<Event>> {
        get_event_by_id(&self.pool, user_id, event_id).await
    }

    pub async fn get_event_by_uid(
        &self,
        user_id: UserId,
        uid: &str,
    ) -> StorageResult<Option<Event>> {
        get_event_by_uid(&self.pool, user_id, uid).await
    }

    pub async fn get_events_by_uids(
        &self,
        user_id: UserId,
        uids: &[&str],
    ) -> StorageResult<Vec<Event>> {
        get_events_by_uids(&self.pool, user_id, uids).await
    }

    pub async fn get_events_by_ids_any(&self, event_ids: &[Uuid]) -> StorageResult<Vec<Event>> {
        get_events_by_ids_any(&self.pool, event_ids).await
    }

    pub async fn get_event_attendees(&self, event_id: Uuid) -> StorageResult<Vec<EventAttendee>> {
        get_event_attendees(&self.pool, event_id).await
    }

    pub async fn get_event_attendees_bulk(
        &self,
        event_ids: &[Uuid],
    ) -> StorageResult<HashMap<Uuid, Vec<EventAttendee>>> {
        get_event_attendees_bulk(&self.pool, event_ids).await
    }

    pub async fn list_pending_invites(
        &self,
        user_id: UserId,
    ) -> StorageResult<Vec<PendingInviteRecord>> {
        list_pending_invites(&self.pool, user_id).await
    }

    pub async fn list_attendees_for_display(
        &self,
        event_id: Uuid,
    ) -> StorageResult<Vec<AttendeeDisplayRecord>> {
        list_attendees_for_display(&self.pool, event_id).await
    }

    pub async fn list_events(
        &self,
        user_id: UserId,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> StorageResult<Vec<Event>> {
        list_events(&self.pool, user_id, start, end, limit, offset).await
    }

    pub async fn list_events_since_sync(
        &self,
        user_id: UserId,
        sync_token: i64,
    ) -> StorageResult<Vec<Event>> {
        list_events_since_sync(&self.pool, user_id, sync_token).await
    }

    pub async fn list_tombstones_since(
        &self,
        user_id: UserId,
        sync_token: i64,
    ) -> StorageResult<Vec<EventTombstone>> {
        list_tombstones_since(&self.pool, user_id, sync_token).await
    }
}

pub struct CalendarTransaction<'a> {
    tx: Transaction<'a, Postgres>,
}

impl CalendarTransaction<'_> {
    pub async fn ensure_user(
        &mut self,
        telegram_id: i64,
        username: Option<&str>,
    ) -> StorageResult<User> {
        self::ensure_user_tx(&mut self.tx, telegram_id, username).await
    }

    pub async fn bump_calendar_state(&mut self, user_id: UserId) -> StorageResult<i64> {
        self::bump_calendar_state_tx(&mut self.tx, user_id).await
    }

    pub async fn get_event_by_id(
        &mut self,
        user_id: UserId,
        event_id: Uuid,
    ) -> StorageResult<Option<Event>> {
        self::get_event_by_id_tx(&mut self.tx, user_id, event_id).await
    }

    pub async fn get_event_by_uid(
        &mut self,
        user_id: UserId,
        uid: &str,
    ) -> StorageResult<Option<Event>> {
        self::get_event_by_uid_tx(&mut self.tx, user_id, uid).await
    }

    pub async fn get_event_by_id_any(&mut self, event_id: Uuid) -> StorageResult<Option<Event>> {
        self::get_event_by_id_any_tx(&mut self.tx, event_id).await
    }

    pub async fn insert_event(&mut self, event: StoredEventWrite) -> StorageResult<Event> {
        self::insert_event_tx(&mut self.tx, event).await
    }

    pub async fn update_event(&mut self, event: StoredEventUpdate) -> StorageResult<Event> {
        self::update_event_tx(&mut self.tx, event).await
    }

    pub async fn set_event_sync_etag(
        &mut self,
        event_id: Uuid,
        user_id: UserId,
        version: i32,
        sync_version: i64,
        etag: String,
    ) -> StorageResult<Event> {
        self::set_event_sync_etag_tx(&mut self.tx, event_id, user_id, version, sync_version, etag)
            .await
    }

    pub async fn delete_event_by_id(
        &mut self,
        user_id: UserId,
        event_id: Uuid,
    ) -> StorageResult<Option<Event>> {
        self::delete_event_by_id_tx(&mut self.tx, user_id, event_id).await
    }

    pub async fn delete_event_by_uid(
        &mut self,
        user_id: UserId,
        uid: &str,
    ) -> StorageResult<Option<Event>> {
        self::delete_event_by_uid_tx(&mut self.tx, user_id, uid).await
    }

    pub async fn insert_tombstone(
        &mut self,
        user_id: UserId,
        uid: &str,
        sync_version: i64,
    ) -> StorageResult<()> {
        self::insert_tombstone_tx(&mut self.tx, user_id, uid, sync_version).await
    }

    pub async fn list_attendees(&mut self, event_id: Uuid) -> StorageResult<Vec<EventAttendee>> {
        self::list_attendees_tx(&mut self.tx, event_id).await
    }

    pub async fn upsert_attendees(
        &mut self,
        event_id: Uuid,
        attendees: &[AttendeeWrite],
    ) -> StorageResult<Vec<AttendeeUpsertResult>> {
        self::upsert_attendees_tx(&mut self.tx, event_id, attendees).await
    }

    pub async fn replace_attendees(
        &mut self,
        event_id: Uuid,
        attendees: &[AttendeeWrite],
    ) -> StorageResult<Vec<AttendeeUpsertResult>> {
        self::replace_attendees_tx(&mut self.tx, event_id, attendees).await
    }

    pub async fn update_attendee_status(
        &mut self,
        event_id: Uuid,
        user_id: i64,
        status: ParticipationStatus,
    ) -> StorageResult<bool> {
        self::update_attendee_status_tx(&mut self.tx, event_id, user_id, status).await
    }

    pub async fn queue_outbox(&mut self, messages: &[OutboxPayload]) -> StorageResult<()> {
        self::queue_outbox_tx(&mut self.tx, messages).await
    }

    pub async fn commit(self) -> StorageResult<()> {
        self.tx.commit().await?;
        Ok(())
    }
}

pub struct StoredEventWrite {
    pub user_id: UserId,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub version: i32,
    pub sync_version: i64,
    pub etag: String,
}

pub struct StoredEventUpdate {
    pub id: Uuid,
    pub user_id: UserId,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub timing: EventTiming,
    pub status: EventStatus,
    pub rrule: Option<String>,
    pub version: i32,
    pub sync_version: i64,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct AttendeeWrite {
    pub email: String,
    pub user_id: Option<i64>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
}

#[derive(Debug, Clone)]
pub struct AttendeeUpsertResult {
    pub email: String,
    pub user_id: Option<i64>,
    pub is_new: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct UserRow {
    pub telegram_id: i64,
    pub telegram_username: Option<String>,
    pub timezone: String,
    pub sync_token: i64,
    pub ctag: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = StorageError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: UserId::new(row.telegram_id),
            telegram_username: row.telegram_username,
            timezone: parse_timezone(&row.timezone)?,
            sync_token: row.sync_token,
            ctag: row.ctag,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EventRow {
    pub id: Uuid,
    pub user_id: i64,
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub status: String,
    pub rrule: Option<String>,
    pub timezone: String,
    pub version: i32,
    pub sync_version: i64,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<EventRow> for Event {
    type Error = StorageError;

    fn try_from(row: EventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            user_id: UserId::new(row.user_id),
            uid: row.uid,
            summary: row.summary,
            description: row.description,
            location: row.location,
            start: row.start,
            end: row.end,
            start_date: row.start_date,
            end_date: row.end_date,
            is_all_day: row.is_all_day,
            status: parse_event_status(&row.status)?,
            rrule: row.rrule,
            timezone: parse_timezone(&row.timezone)?,
            version: row.version,
            sync_version: row.sync_version,
            etag: row.etag,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EventAttendeeRow {
    pub event_id: Uuid,
    pub email: String,
    pub user_id: Option<i64>,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<EventAttendeeRow> for EventAttendee {
    type Error = StorageError;

    fn try_from(row: EventAttendeeRow) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: row.event_id,
            email: row.email,
            user_id: row.user_id,
            role: parse_attendee_role(&row.role)?,
            status: parse_participation_status(&row.status)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingInviteRecord {
    pub event_id: Uuid,
    pub summary: String,
    pub start: Option<DateTime<Utc>>,
    pub start_date: Option<NaiveDate>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub organizer_username: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttendeeDisplayRecord {
    pub email: String,
    pub telegram_id: Option<i64>,
    pub role: String,
    pub status: String,
    pub telegram_username: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EventTombstoneRow {
    pub user_id: i64,
    pub uid: String,
    pub sync_version: i64,
    pub deleted_at: DateTime<Utc>,
}

impl From<EventTombstoneRow> for EventTombstone {
    fn from(row: EventTombstoneRow) -> Self {
        Self {
            user_id: UserId::new(row.user_id),
            uid: row.uid,
            sync_version: row.sync_version,
            deleted_at: row.deleted_at,
        }
    }
}

fn parse_timezone(value: &str) -> StorageResult<Timezone> {
    Timezone::parse(value).map_err(|err| StorageError::InvalidData(err.to_string()))
}

fn parse_event_status(value: &str) -> StorageResult<EventStatus> {
    match value {
        "CONFIRMED" => Ok(EventStatus::Confirmed),
        "TENTATIVE" => Ok(EventStatus::Tentative),
        "CANCELLED" => Ok(EventStatus::Cancelled),
        other => Err(StorageError::InvalidData(format!(
            "unknown event_status: {other}"
        ))),
    }
}

fn parse_attendee_role(value: &str) -> StorageResult<AttendeeRole> {
    match value {
        "ORGANIZER" => Ok(AttendeeRole::Organizer),
        "ATTENDEE" => Ok(AttendeeRole::Attendee),
        other => Err(StorageError::InvalidData(format!(
            "unknown attendee_role: {other}"
        ))),
    }
}

fn parse_participation_status(value: &str) -> StorageResult<ParticipationStatus> {
    match value {
        "NEEDS-ACTION" => Ok(ParticipationStatus::NeedsAction),
        "ACCEPTED" => Ok(ParticipationStatus::Accepted),
        "DECLINED" => Ok(ParticipationStatus::Declined),
        "TENTATIVE" => Ok(ParticipationStatus::Tentative),
        other => Err(StorageError::InvalidData(format!(
            "unknown attendee_status: {other}"
        ))),
    }
}

fn optional_user(row: Option<UserRow>) -> StorageResult<Option<User>> {
    row.map(User::try_from).transpose()
}

fn optional_event(row: Option<EventRow>) -> StorageResult<Option<Event>> {
    row.map(Event::try_from).transpose()
}

fn event_rows(rows: Vec<EventRow>) -> StorageResult<Vec<Event>> {
    rows.into_iter().map(Event::try_from).collect()
}

fn attendee_rows(rows: Vec<EventAttendeeRow>) -> StorageResult<Vec<EventAttendee>> {
    rows.into_iter().map(EventAttendee::try_from).collect()
}

pub(crate) async fn ensure_user_tx(
    conn: &mut PgConnection,
    telegram_id: i64,
    username: Option<&str>,
) -> StorageResult<User> {
    let query = format!(
        r#"
        INSERT INTO users (telegram_id, telegram_username)
        VALUES ($1, $2)
        ON CONFLICT (telegram_id) DO UPDATE
        SET telegram_username = COALESCE(EXCLUDED.telegram_username, users.telegram_username)
        RETURNING {USER_COLUMNS}
        "#,
    );

    let user = sqlx::query_as::<_, UserRow>(&query)
        .bind(telegram_id)
        .bind(username)
        .fetch_one(conn)
        .await?;

    User::try_from(user)
}

async fn get_user_by_id(pool: &PgPool, user_id: UserId) -> StorageResult<Option<User>> {
    let query = format!("SELECT {USER_COLUMNS} FROM users WHERE telegram_id = $1");
    let user = sqlx::query_as::<_, UserRow>(&query)
        .bind(user_id.inner())
        .fetch_optional(pool)
        .await?;

    optional_user(user)
}

async fn get_user_by_username(pool: &PgPool, username: &str) -> StorageResult<Option<User>> {
    let query =
        format!("SELECT {USER_COLUMNS} FROM users WHERE lower(telegram_username) = lower($1)");
    let user = sqlx::query_as::<_, UserRow>(&query)
        .bind(username)
        .fetch_optional(pool)
        .await?;

    optional_user(user)
}

async fn bump_calendar_state_tx(conn: &mut PgConnection, user_id: UserId) -> StorageResult<i64> {
    let sync_version = sqlx::query_scalar::<_, i64>(
        r#"
        UPDATE users
        SET sync_token = sync_token + 1,
            ctag = sync_token + 1,
            updated_at = NOW()
        WHERE telegram_id = $1
        RETURNING sync_token
        "#,
    )
    .bind(user_id.inner())
    .fetch_one(conn)
    .await?;

    Ok(sync_version)
}

async fn get_event_by_id_tx(
    conn: &mut PgConnection,
    user_id: UserId,
    event_id: Uuid,
) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE id = $1 AND user_id = $2");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .bind(user_id.inner())
        .fetch_optional(conn)
        .await?;

    optional_event(event)
}

async fn get_event_by_uid_tx(
    conn: &mut PgConnection,
    user_id: UserId,
    uid: &str,
) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE user_id = $1 AND uid = $2");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(user_id.inner())
        .bind(uid)
        .fetch_optional(conn)
        .await?;

    optional_event(event)
}

async fn get_event_by_id_any_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE id = $1");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .fetch_optional(conn)
        .await?;

    optional_event(event)
}

async fn get_event_by_id(
    pool: &PgPool,
    user_id: UserId,
    event_id: Uuid,
) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE id = $1 AND user_id = $2");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .bind(user_id.inner())
        .fetch_optional(pool)
        .await?;

    optional_event(event)
}

async fn get_event_by_uid(
    pool: &PgPool,
    user_id: UserId,
    uid: &str,
) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE user_id = $1 AND uid = $2");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(user_id.inner())
        .bind(uid)
        .fetch_optional(pool)
        .await?;

    optional_event(event)
}

async fn get_events_by_uids(
    pool: &PgPool,
    user_id: UserId,
    uids: &[&str],
) -> StorageResult<Vec<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE user_id = $1 AND uid = ANY($2)");
    let events = sqlx::query_as::<_, EventRow>(&query)
        .bind(user_id.inner())
        .bind(uids)
        .fetch_all(pool)
        .await?;

    event_rows(events)
}

async fn get_events_by_ids_any(pool: &PgPool, event_ids: &[Uuid]) -> StorageResult<Vec<Event>> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }

    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE id = ANY($1)");
    let events = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_ids)
        .fetch_all(pool)
        .await?;

    event_rows(events)
}

async fn get_event_by_id_any(pool: &PgPool, event_id: Uuid) -> StorageResult<Option<Event>> {
    let query = format!("SELECT {EVENT_COLUMNS} FROM events WHERE id = $1");
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .fetch_optional(pool)
        .await?;

    optional_event(event)
}

async fn get_event_attendees(pool: &PgPool, event_id: Uuid) -> StorageResult<Vec<EventAttendee>> {
    let query = format!("SELECT {ATTENDEE_COLUMNS} FROM event_attendees WHERE event_id = $1");
    let attendees = sqlx::query_as::<_, EventAttendeeRow>(&query)
        .bind(event_id)
        .fetch_all(pool)
        .await?;

    attendee_rows(attendees)
}

async fn get_event_attendees_bulk(
    pool: &PgPool,
    event_ids: &[Uuid],
) -> StorageResult<HashMap<Uuid, Vec<EventAttendee>>> {
    if event_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let query = format!(
        r#"
        SELECT {ATTENDEE_COLUMNS}
        FROM event_attendees
        WHERE event_id = ANY($1)
        ORDER BY event_id, email
        "#,
    );
    let attendees = sqlx::query_as::<_, EventAttendeeRow>(&query)
        .bind(event_ids)
        .fetch_all(pool)
        .await?;

    let mut grouped: HashMap<Uuid, Vec<EventAttendee>> = HashMap::new();
    for attendee in attendees {
        let attendee = EventAttendee::try_from(attendee)?;
        grouped.entry(attendee.event_id).or_default().push(attendee);
    }

    Ok(grouped)
}

async fn list_pending_invites(
    pool: &PgPool,
    user_id: UserId,
) -> StorageResult<Vec<PendingInviteRecord>> {
    let invites = sqlx::query_as::<_, PendingInviteRecord>(
        r#"
        SELECT e.id AS event_id, e.summary, e.start, e.start_date, e.is_all_day, e.location,
               u.telegram_username AS organizer_username
        FROM event_attendees ea
        JOIN events e ON ea.event_id = e.id
        JOIN users u ON e.user_id = u.telegram_id
        WHERE ea.user_id = $1
          AND ea.status = 'NEEDS-ACTION'
        ORDER BY COALESCE(e.start, (e.start_date AT TIME ZONE 'UTC')) ASC
        "#,
    )
    .bind(user_id.inner())
    .fetch_all(pool)
    .await?;

    Ok(invites)
}

async fn list_attendees_for_display(
    pool: &PgPool,
    event_id: Uuid,
) -> StorageResult<Vec<AttendeeDisplayRecord>> {
    let attendees = sqlx::query_as::<_, AttendeeDisplayRecord>(
        r#"
        SELECT ea.email, ea.user_id AS telegram_id, ea.role::text AS role, ea.status::text AS status,
               u.telegram_username
        FROM event_attendees ea
        LEFT JOIN users u ON ea.user_id = u.telegram_id
        WHERE ea.event_id = $1
        ORDER BY
            CASE ea.role::text
                WHEN 'ORGANIZER' THEN 0
                ELSE 1
            END,
            ea.created_at ASC
        "#,
    )
    .bind(event_id)
    .fetch_all(pool)
    .await?;

    Ok(attendees)
}

async fn list_events(
    pool: &PgPool,
    user_id: UserId,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> StorageResult<Vec<Event>> {
    let offset = offset.unwrap_or(0);

    let events = match (start, end) {
        (Some(start_time), Some(end_time)) => {
            let start_date = start_time.date_naive();
            let end_date = end_time.date_naive();
            let query = format!(
                r#"
                SELECT {EVENT_COLUMNS} FROM events
                WHERE user_id = $1
                AND (
                    (is_all_day = false AND start >= $2 AND start < $3)
                    OR
                    (is_all_day = true AND start_date >= $4 AND start_date < $5)
                )
                ORDER BY COALESCE(start, start_date::timestamp AT TIME ZONE 'UTC') ASC
                LIMIT $6 OFFSET $7
                "#,
            );
            sqlx::query_as::<_, EventRow>(&query)
                .bind(user_id.inner())
                .bind(start_time)
                .bind(end_time)
                .bind(start_date)
                .bind(end_date)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        }
        _ => {
            let query = format!(
                r#"
                SELECT {EVENT_COLUMNS} FROM events
                WHERE user_id = $1
                ORDER BY COALESCE(start, start_date::timestamp AT TIME ZONE 'UTC') ASC
                LIMIT $2 OFFSET $3
                "#,
            );
            sqlx::query_as::<_, EventRow>(&query)
                .bind(user_id.inner())
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await?
        }
    };

    event_rows(events)
}

async fn list_events_since_sync(
    pool: &PgPool,
    user_id: UserId,
    sync_token: i64,
) -> StorageResult<Vec<Event>> {
    let query = format!(
        r#"
        SELECT {EVENT_COLUMNS} FROM events
        WHERE user_id = $1
        AND sync_version > $2
        ORDER BY sync_version ASC
        "#,
    );
    let events = sqlx::query_as::<_, EventRow>(&query)
        .bind(user_id.inner())
        .bind(sync_token)
        .fetch_all(pool)
        .await?;

    event_rows(events)
}

async fn insert_event_tx(conn: &mut PgConnection, event: StoredEventWrite) -> StorageResult<Event> {
    let TimingColumns {
        start,
        end,
        start_date,
        end_date,
        is_all_day,
        timezone,
    } = TimingColumns::from_timing(&event.timing);

    let query = format!(
        r#"
        INSERT INTO events (
            user_id, uid, summary, description, location,
            start, "end", start_date, end_date, is_all_day,
            status, timezone, rrule, version, sync_version, etag
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10,
            $11::text::event_status, $12, $13, $14, $15, $16
        )
        RETURNING {EVENT_COLUMNS}
        "#,
    );
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event.user_id.inner())
        .bind(event.uid)
        .bind(event.summary)
        .bind(event.description)
        .bind(event.location)
        .bind(start)
        .bind(end)
        .bind(start_date)
        .bind(end_date)
        .bind(is_all_day)
        .bind(event.status.as_sql())
        .bind(timezone)
        .bind(event.rrule)
        .bind(event.version)
        .bind(event.sync_version)
        .bind(event.etag)
        .fetch_one(conn)
        .await?;

    Event::try_from(event)
}

async fn update_event_tx(
    conn: &mut PgConnection,
    event: StoredEventUpdate,
) -> StorageResult<Event> {
    let TimingColumns {
        start,
        end,
        start_date,
        end_date,
        is_all_day,
        timezone,
    } = TimingColumns::from_timing(&event.timing);

    let query = format!(
        r#"
        UPDATE events
        SET summary = $3,
            description = $4,
            location = $5,
            start = $6,
            "end" = $7,
            start_date = $8,
            end_date = $9,
            is_all_day = $10,
            status = $11::text::event_status,
            timezone = $12,
            rrule = $13,
            version = $14,
            sync_version = $15,
            etag = $16,
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING {EVENT_COLUMNS}
        "#,
    );
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event.id)
        .bind(event.user_id.inner())
        .bind(event.summary)
        .bind(event.description)
        .bind(event.location)
        .bind(start)
        .bind(end)
        .bind(start_date)
        .bind(end_date)
        .bind(is_all_day)
        .bind(event.status.as_sql())
        .bind(timezone)
        .bind(event.rrule)
        .bind(event.version)
        .bind(event.sync_version)
        .bind(event.etag)
        .fetch_one(conn)
        .await?;

    Event::try_from(event)
}

async fn set_event_sync_etag_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
    user_id: UserId,
    version: i32,
    sync_version: i64,
    etag: String,
) -> StorageResult<Event> {
    let query = format!(
        r#"
        UPDATE events
        SET version = $3,
            sync_version = $4,
            etag = $5,
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING {EVENT_COLUMNS}
        "#,
    );
    let event = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .bind(user_id.inner())
        .bind(version)
        .bind(sync_version)
        .bind(etag)
        .fetch_one(conn)
        .await?;

    Event::try_from(event)
}

async fn delete_event_by_id_tx(
    conn: &mut PgConnection,
    user_id: UserId,
    event_id: Uuid,
) -> StorageResult<Option<Event>> {
    let query =
        format!("DELETE FROM events WHERE id = $1 AND user_id = $2 RETURNING {EVENT_COLUMNS}");
    let deleted = sqlx::query_as::<_, EventRow>(&query)
        .bind(event_id)
        .bind(user_id.inner())
        .fetch_optional(conn)
        .await?;

    optional_event(deleted)
}

async fn delete_event_by_uid_tx(
    conn: &mut PgConnection,
    user_id: UserId,
    uid: &str,
) -> StorageResult<Option<Event>> {
    let query =
        format!("DELETE FROM events WHERE user_id = $1 AND uid = $2 RETURNING {EVENT_COLUMNS}");
    let deleted = sqlx::query_as::<_, EventRow>(&query)
        .bind(user_id.inner())
        .bind(uid)
        .fetch_optional(conn)
        .await?;

    optional_event(deleted)
}

async fn insert_tombstone_tx(
    conn: &mut PgConnection,
    user_id: UserId,
    uid: &str,
    sync_version: i64,
) -> StorageResult<()> {
    sqlx::query(
        r#"
        INSERT INTO event_tombstones (user_id, uid, sync_version, deleted_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id, uid) DO UPDATE
        SET sync_version = EXCLUDED.sync_version,
            deleted_at = EXCLUDED.deleted_at
        "#,
    )
    .bind(user_id.inner())
    .bind(uid)
    .bind(sync_version)
    .execute(conn)
    .await?;

    Ok(())
}

async fn list_attendees_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> StorageResult<Vec<EventAttendee>> {
    let query = format!(
        r#"
        SELECT {ATTENDEE_COLUMNS}
        FROM event_attendees
        WHERE event_id = $1
        ORDER BY email
        "#,
    );
    let attendees = sqlx::query_as::<_, EventAttendeeRow>(&query)
        .bind(event_id)
        .fetch_all(conn)
        .await?;

    attendee_rows(attendees)
}

async fn upsert_attendees_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
    attendees: &[AttendeeWrite],
) -> StorageResult<Vec<AttendeeUpsertResult>> {
    if attendees.is_empty() {
        return Ok(Vec::new());
    }

    let mut builder: QueryBuilder<Postgres> =
        QueryBuilder::new("INSERT INTO event_attendees (event_id, user_id, email, role, status) ");

    builder.push_values(attendees, |mut row, attendee| {
        row.push_bind(event_id);
        row.push_bind(attendee.user_id);
        row.push_bind(&attendee.email);
        row.push_bind(attendee.role.as_sql())
            .push("::text::attendee_role");
        row.push_bind(attendee.status.as_sql())
            .push("::text::attendee_status");
    });

    builder.push(
        r#"
        ON CONFLICT (event_id, email) DO UPDATE
        SET user_id = EXCLUDED.user_id,
            role = EXCLUDED.role,
            status = EXCLUDED.status,
            updated_at = NOW()
        RETURNING email, user_id, (xmax = 0) AS is_new
        "#,
    );

    let rows = builder.build().fetch_all(conn).await?;
    let results = rows
        .into_iter()
        .map(|row| AttendeeUpsertResult {
            email: row.get("email"),
            user_id: row.get("user_id"),
            is_new: row.get("is_new"),
        })
        .collect();

    Ok(results)
}

async fn replace_attendees_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
    attendees: &[AttendeeWrite],
) -> StorageResult<Vec<AttendeeUpsertResult>> {
    if attendees.is_empty() {
        sqlx::query("DELETE FROM event_attendees WHERE event_id = $1")
            .bind(event_id)
            .execute(&mut *conn)
            .await?;
        return Ok(Vec::new());
    }

    let attendee_emails = attendees
        .iter()
        .map(|attendee| attendee.email.clone())
        .collect::<Vec<_>>();

    sqlx::query(
        r#"
        DELETE FROM event_attendees
        WHERE event_id = $1
          AND email <> ALL($2::text[])
        "#,
    )
    .bind(event_id)
    .bind(&attendee_emails)
    .execute(&mut *conn)
    .await?;

    upsert_attendees_tx(conn, event_id, attendees).await
}

async fn update_attendee_status_tx(
    conn: &mut PgConnection,
    event_id: Uuid,
    user_id: i64,
    status: ParticipationStatus,
) -> StorageResult<bool> {
    let result = sqlx::query(
        r#"
        UPDATE event_attendees
        SET status = $3::text::attendee_status,
            updated_at = NOW()
        WHERE event_id = $1 AND user_id = $2
        "#,
    )
    .bind(event_id)
    .bind(user_id)
    .bind(status.as_sql())
    .execute(conn)
    .await?;

    Ok(result.rows_affected() > 0)
}

async fn queue_outbox_tx(conn: &mut PgConnection, messages: &[OutboxPayload]) -> StorageResult<()> {
    if messages.is_empty() {
        return Ok(());
    }

    let rows = messages
        .iter()
        .map(|payload| {
            Ok((
                payload.kind().as_str().to_string(),
                payload.payload_json()?,
                payload.dedupe_key(),
            ))
        })
        .collect::<StorageResult<Vec<_>>>()?;

    let mut builder: QueryBuilder<Postgres> =
        QueryBuilder::new("INSERT INTO outbox_messages (kind, payload, dedupe_key) ");

    builder.push_values(rows, |mut row, (kind, payload, dedupe_key)| {
        row.push_bind(kind);
        row.push_bind(payload);
        row.push_bind(dedupe_key);
    });

    builder.push(" ON CONFLICT (dedupe_key) WHERE dedupe_key IS NOT NULL DO NOTHING");
    builder.build().execute(conn).await?;

    Ok(())
}

async fn list_tombstones_since(
    pool: &PgPool,
    user_id: UserId,
    sync_token: i64,
) -> StorageResult<Vec<EventTombstone>> {
    let tombstones = sqlx::query_as::<_, EventTombstoneRow>(
        r#"
        SELECT user_id, uid, sync_version, deleted_at
        FROM event_tombstones
        WHERE user_id = $1
          AND sync_version > $2
        ORDER BY sync_version ASC
        "#,
    )
    .bind(user_id.inner())
    .bind(sync_token)
    .fetch_all(pool)
    .await?;

    Ok(tombstones.into_iter().map(Into::into).collect())
}

struct TimingColumns {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    is_all_day: bool,
    timezone: String,
}

impl TimingColumns {
    fn from_timing(timing: &EventTiming) -> Self {
        match timing {
            EventTiming::Timed {
                start,
                end,
                timezone,
            } => Self {
                start: Some(*start),
                end: Some(*end),
                start_date: None,
                end_date: None,
                is_all_day: false,
                timezone: timezone.as_str().to_string(),
            },
            EventTiming::AllDay {
                start_date,
                end_date,
            } => Self {
                start: None,
                end: None,
                start_date: Some(*start_date),
                end_date: Some(*end_date),
                is_all_day: true,
                timezone: "UTC".to_string(),
            },
        }
    }
}
