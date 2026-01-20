# Televent Development Roadmap - Path to Production

**Last Updated**: 2026-01-20
**Status**: Phase 1 Complete, Phase 2-4 Remaining

---

## 📋 **Project Vision**

Create a **Telegram-first calendar** that:
1. ✅ Manages events via Telegram bot
2. ✅ Syncs to desktop calendar apps (Thunderbird, Apple Calendar) via CalDAV
3. ❌ **Invites Gmail/Outlook users** to events (send iCalendar invites via email)
4. ❌ **Receives invites** from Gmail/Outlook users (process iCalendar emails)
5. ⏳ Telegram miniapp with calendar UI (future enhancement)

---

## 🎯 **Current State Assessment**

### ✅ **What's Working (Phase 1 - COMPLETE)**

| Feature | Status | Notes |
|---------|--------|-------|
| Telegram bot | ✅ Working | 11 commands implemented |
| CalDAV server | ✅ Working | RFC 4791 compliant, read/write |
| Device passwords | ✅ Working | Argon2id auth for CalDAV clients |
| Event CRUD | ✅ Working | Create, read, update, delete events |
| Background worker | ✅ Working | Outbox pattern with retry logic |
| Email sending | ✅ Working | Basic SMTP via lettre (plain text only) |
| PostgreSQL schema | ✅ Working | Proper indexes, triggers, constraints |
| Recurrence rules | ✅ Working | Basic RRULE support (DAILY, WEEKLY, etc.) |
| Timezone handling | ✅ Working | chrono-tz integration |

### ❌ **Critical Gaps (Blocks Production Use)**

| Feature | Status | Impact | Priority |
|---------|--------|--------|----------|
| **Invite system** | ❌ Missing | Can't invite Gmail/Outlook users | **P0** |
| **Attendee tracking** | ❌ Missing | No RSVP, no participant list | **P0** |
| **iCalendar email** | ❌ Missing | No VEVENT attachments in emails | **P0** |
| **Email invite parsing** | ❌ Missing | Can't receive invites from others | **P0** |
| Rate limiting | ❌ Missing | Security vulnerability | **P0** |
| Deleted event tracking | ❌ Missing | CalDAV sync incomplete (RFC 6578) | **P1** |
| ICS export | ❌ Stub | Advertised but not working | **P1** |

---

## 🚧 **Development Phases**

## **Phase 2: Invite & Attendee System** 🔴 CRITICAL

**Goal**: Enable inviting Gmail/Outlook users and tracking RSVPs

**Duration**: 2-3 weeks
**Priority**: P0 - Blocks core user value

### **2.1 Database Schema Changes**

**Migration: `20260121_create_attendees_table.sql`**
```sql
-- Store event attendees (organizer + invited participants)
CREATE TABLE event_attendees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    display_name TEXT,
    role TEXT NOT NULL CHECK (role IN ('ORGANIZER', 'ATTENDEE')),
    status TEXT NOT NULL DEFAULT 'NEEDS-ACTION'
        CHECK (status IN ('NEEDS-ACTION', 'ACCEPTED', 'DECLINED', 'TENTATIVE')),
    rsvp_required BOOLEAN NOT NULL DEFAULT true,
    telegram_id BIGINT REFERENCES users(telegram_id),  -- NULL if external user
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, email)  -- One RSVP per email per event
);

CREATE INDEX idx_attendees_event ON event_attendees(event_id);
CREATE INDEX idx_attendees_email ON event_attendees(email);
CREATE INDEX idx_attendees_telegram ON event_attendees(telegram_id) WHERE telegram_id IS NOT NULL;

-- Store event organizer (denormalized for performance)
ALTER TABLE events ADD COLUMN organizer_email TEXT;
ALTER TABLE events ADD COLUMN organizer_name TEXT;

-- Add attendee count to events table
ALTER TABLE events ADD COLUMN attendee_count INTEGER DEFAULT 0;
```

### **2.2 Core Models** (`crates/core/src/models.rs`)

```rust
/// Event attendee (participant in an event)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventAttendee {
    pub id: Uuid,
    pub event_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub role: AttendeeRole,
    pub status: ParticipationStatus,
    pub rsvp_required: bool,
    pub telegram_id: Option<i64>,  // Linked Televent user
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum AttendeeRole {
    #[sqlx(rename = "ORGANIZER")]
    Organizer,
    #[sqlx(rename = "ATTENDEE")]
    Attendee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum ParticipationStatus {
    #[sqlx(rename = "NEEDS-ACTION")]
    NeedsAction,
    #[sqlx(rename = "ACCEPTED")]
    Accepted,
    #[sqlx(rename = "DECLINED")]
    Declined,
    #[sqlx(rename = "TENTATIVE")]
    Tentative,
}
```

### **2.3 Bot Commands** (`crates/bot/src/handlers.rs`)

**Add new commands**:
```rust
/invite <event_id> <email1> <email2> ...  // Invite external users to event
/rsvp accept <event_id>                    // Accept event invitation
/rsvp decline <event_id>                   // Decline event invitation
/rsvp tentative <event_id>                 // Mark as tentative
/attendees <event_id>                      // Show attendee list with RSVP status
```

**Update `/create` command**:
- After creating event, ask: "Invite others? Send email addresses (separated by spaces)"
- Store organizer (bot user's email or telegram_id@televent.app)

### **2.4 iCalendar Email Support** (`crates/worker/src/mailer.rs`)

**Enhance mailer to send VEVENT attachments**:

```rust
use lettre::message::{Attachment, MultiPart, SinglePart};

/// Send calendar invite via email with iCalendar attachment
pub async fn send_calendar_invite(
    to_emails: &[String],
    event: &Event,
    attendees: &[EventAttendee],
    method: CalendarMethod,  // REQUEST | REPLY | CANCEL
) -> Result<()> {
    let ical_content = generate_vevent_with_attendees(event, attendees, method);

    let email = Message::builder()
        .from("noreply@televent.app".parse().unwrap())
        .to(to_emails[0].parse().unwrap())
        .subject(&event.summary)
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::plain(format!(
                    "You've been invited to: {}\n\nTime: {} - {}\nLocation: {}\n",
                    event.summary,
                    event.start.format("%Y-%m-%d %H:%M"),
                    event.end.format("%H:%M"),
                    event.location.as_deref().unwrap_or("N/A")
                )))
                .singlepart(
                    Attachment::new("invite.ics".to_string())
                        .body(ical_content.into_bytes(), "text/calendar".parse().unwrap())
                )
        )?;

    // Send via SMTP...
}

/// Generate iCalendar VEVENT with ATTENDEE properties
fn generate_vevent_with_attendees(
    event: &Event,
    attendees: &[EventAttendee],
    method: CalendarMethod,
) -> String {
    format!(
        "BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Televent//Calendar 1.0//EN\r
METHOD:{}\r
BEGIN:VEVENT\r
UID:{}\r
DTSTAMP:{}\r
DTSTART:{}\r
DTEND:{}\r
SUMMARY:{}\r
DESCRIPTION:{}\r
LOCATION:{}\r
ORGANIZER;CN={}:mailto:{}\r
{}\
STATUS:{}\r
SEQUENCE:{}\r
END:VEVENT\r
END:VCALENDAR\r
",
        method.as_str(),
        event.uid,
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        event.start.format("%Y%m%dT%H%M%SZ"),
        event.end.format("%Y%m%dT%H%M%SZ"),
        event.summary,
        event.description.as_deref().unwrap_or(""),
        event.location.as_deref().unwrap_or(""),
        event.organizer_name.as_deref().unwrap_or("Organizer"),
        event.organizer_email.as_deref().unwrap_or("noreply@televent.app"),
        attendees.iter()
            .map(|a| format!(
                "ATTENDEE;CN={};PARTSTAT={};RSVP={}:mailto:{}\r\n",
                a.display_name.as_deref().unwrap_or(&a.email),
                a.status.as_str(),
                if a.rsvp_required { "TRUE" } else { "FALSE" },
                a.email
            ))
            .collect::<String>(),
        event.status.as_str(),
        event.version
    )
}
```

### **2.5 Outbox Integration**

**Add new message types**:
```rust
// crates/worker/src/processors.rs
match message.message_type.as_str() {
    "calendar_invite" => send_calendar_invite_email(message).await,
    "calendar_update" => send_calendar_update_email(message).await,
    "calendar_cancel" => send_calendar_cancel_email(message).await,
    // ... existing handlers
}
```

**Trigger on event creation with attendees**:
```rust
// crates/bot/src/handlers.rs - handle_create()
if !invite_emails.is_empty() {
    sqlx::query(
        "INSERT INTO outbox_messages (message_type, payload, status)
         VALUES ('calendar_invite', $1, 'pending')"
    )
    .bind(json!({
        "event_id": event_id,
        "to_emails": invite_emails,
        "organizer_telegram_id": telegram_id
    }))
    .execute(&db)
    .await?;
}
```

---

## **Phase 3: Incoming Invite Processing** 🟡 HIGH PRIORITY

**Goal**: Receive and parse calendar invites from Gmail/Outlook users

**Duration**: 2 weeks
**Priority**: P0 - Required for bi-directional invites

### **3.1 Email Receiving Options**

**Option A: Mailgun/SendGrid Inbound Webhooks** (Recommended)
- Use Mailgun's inbound email API
- Parse webhooks with multipart/alternative + text/calendar
- Low latency, production-ready

**Option B: IMAP Polling**
- Poll dedicated inbox (e.g., invites@televent.app)
- Parse emails with iCalendar attachments
- Simpler but higher latency

**Implementation** (`crates/worker/src/email_parser.rs`):
```rust
use icalendar::parser::unfold;

/// Parse incoming email for iCalendar invites
pub async fn parse_inbound_email(
    raw_email: &str,
) -> Result<Option<IncomingInvite>> {
    // Extract text/calendar part from multipart email
    let ical_part = extract_ical_from_email(raw_email)?;

    // Parse VEVENT
    let vevent = parse_vevent(&ical_part)?;

    // Extract organizer and attendees
    let organizer = vevent.get_property("ORGANIZER")
        .and_then(|p| extract_email_from_mailto(p.value()));

    let attendees: Vec<_> = vevent.get_properties("ATTENDEE")
        .map(|p| parse_attendee_property(p))
        .collect();

    Ok(Some(IncomingInvite {
        method: vevent.get_property("METHOD")?.value().to_string(),
        uid: vevent.uid()?,
        organizer,
        attendees,
        event_data: parse_event_from_vevent(vevent)?,
    }))
}
```

### **3.2 Invite Matching Logic**

**Match incoming invite to Telegram user**:
```sql
-- Find if any attendee email matches a Televent user
SELECT u.telegram_id, u.id, ea.email
FROM event_attendees ea
JOIN users u ON ea.email = u.email OR ea.telegram_id = u.telegram_id
WHERE ea.event_id = $1;
```

**Notify user via Telegram**:
```rust
// crates/worker/src/processors.rs
async fn process_incoming_invite(invite: IncomingInvite) {
    // Find matching Telegram users
    let telegram_users = find_users_by_email(&invite.attendees).await?;

    for user in telegram_users {
        // Send notification via bot
        send_telegram_notification(
            user.telegram_id,
            format!(
                "📧 New event invitation!\n\n\
                 📅 {}\n\
                 🕐 {}\n\
                 👤 Organized by {}\n\n\
                 Use /rsvp accept {} to accept",
                invite.summary,
                invite.start.format("%Y-%m-%d %H:%M"),
                invite.organizer_name,
                invite.uid
            )
        ).await?;

        // Auto-create event in user's calendar (status: TENTATIVE)
        create_event_from_invite(&user, &invite).await?;
    }
}
```

### **3.3 RSVP Reply Emails**

**When user runs `/rsvp accept`**:
```rust
// Update attendee status in DB
sqlx::query(
    "UPDATE event_attendees
     SET status = 'ACCEPTED', updated_at = NOW()
     WHERE event_id = $1 AND telegram_id = $2"
)
.bind(event_id)
.bind(telegram_id)
.execute(&db)
.await?;

// Send REPLY email to organizer
queue_outbox_message("calendar_reply", json!({
    "event_uid": event.uid,
    "organizer_email": event.organizer_email,
    "attendee_email": user.email,
    "partstat": "ACCEPTED"
}));
```

**Email format (METHOD:REPLY)**:
```ics
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Televent//Calendar 1.0//EN
METHOD:REPLY
BEGIN:VEVENT
UID:original-event-uid-123
ATTENDEE;CN=John Doe;PARTSTAT=ACCEPTED:mailto:john@example.com
ORGANIZER:mailto:organizer@gmail.com
DTSTAMP:20260120T120000Z
SEQUENCE:0
END:VEVENT
END:VCALENDAR
```

---

## **Phase 4: Production Hardening** 🟠 REQUIRED

**Duration**: 1 week
**Priority**: P0 - Security & stability

### **4.1 Rate Limiting** (`crates/api/src/middleware/rate_limit.rs`)

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use std::time::Duration;

pub fn create_rate_limiter() -> GovernorLayer<PeerIpKeyExtractor> {
    let config = Box::new(
        GovernorConfigBuilder::default()
            .per_second(10)  // 10 requests per second per IP
            .burst_size(30)  // Allow bursts up to 30
            .finish()
            .unwrap()
    );

    GovernorLayer {
        config: Box::leak(config),
    }
}

// Apply to API routes
app.layer(create_rate_limiter())
```

### **4.2 Deleted Event Tracking**

**Migration: `20260122_deleted_events.sql`**
```sql
CREATE TABLE deleted_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id UUID NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    uid TEXT NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sync_token TEXT NOT NULL  -- Sync token when deleted
);

CREATE INDEX idx_deleted_events_calendar ON deleted_events(calendar_id, deleted_at);
CREATE UNIQUE INDEX idx_deleted_events_uid ON deleted_events(calendar_id, uid);
```

**Update CalDAV DELETE handler**:
```rust
// crates/api/src/routes/caldav.rs
async fn handle_delete_event(...) {
    let event = get_event_by_uid(...).await?;

    // Insert into deleted_events BEFORE deleting
    sqlx::query(
        "INSERT INTO deleted_events (calendar_id, uid, sync_token)
         VALUES ($1, $2, (SELECT sync_token FROM calendars WHERE id = $1))"
    )
    .bind(calendar_id)
    .bind(&event.uid)
    .execute(&pool)
    .await?;

    // Now delete the event
    sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(event.id)
        .execute(&pool)
        .await?;
}
```

### **4.3 Email Validation & User Lookup**

**Add email to users table**:
```sql
ALTER TABLE users ADD COLUMN email TEXT UNIQUE;
CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;
```

**Update `/start` command**:
```rust
// Ask user for email on first registration
bot.send_message(chat_id,
    "To receive calendar invites from Gmail/Outlook users, please provide your email:"
).await?;

// Store in users.email field
```

---

## **Phase 5: UX Enhancements** 🟢 POLISH

**Duration**: 1 week
**Priority**: P1 - User experience

### **5.1 Enhanced Bot Event Display**

```rust
// Show end times, descriptions, attendee counts
fn format_event_details(event: &Event, attendee_count: i32) -> String {
    format!(
        "📅 <b>{}</b>\n\
         🕐 {} - {}\n\
         📍 {}\n\
         👥 {} attendee(s)\n\
         📝 {}\n",
        event.summary,
        event.start.format("%a, %b %d at %H:%M"),
        event.end.format("%H:%M"),
        event.location.as_deref().unwrap_or("No location"),
        attendee_count,
        event.description.as_deref().unwrap_or("No description")
    )
}
```

### **5.2 ICS Export Implementation**

```rust
// crates/bot/src/handlers.rs
pub async fn handle_export(bot: Bot, msg: Message, db: BotDb) -> Result<()> {
    let events = db.get_user_events(telegram_id, /* all future events */).await?;

    let ics_content = generate_ics_calendar(&events);
    let file = InputFile::memory(ics_content.into_bytes())
        .file_name("my_calendar.ics");

    bot.send_document(msg.chat.id, file)
        .caption("📥 Your calendar export (import into any calendar app)")
        .await?;

    Ok(())
}
```

### **5.3 Notification Preferences**

```sql
-- Add to user_preferences table
ALTER TABLE user_preferences ADD COLUMN notify_on_invite BOOLEAN DEFAULT true;
ALTER TABLE user_preferences ADD COLUMN notify_before_event INTERVAL DEFAULT '15 minutes';
```

---

## **Phase 6: Testing & Documentation** 🔵 REQUIRED

**Duration**: 1 week
**Priority**: P1 - Quality assurance

### **6.1 Integration Tests**

```bash
cargo add --dev testcontainers testcontainers-modules
```

**Test scenarios**:
- CalDAV sync with Thunderbird (automated)
- Email invite sending and parsing
- RSVP workflow (accept/decline/tentative)
- Rate limiting under load

### **6.2 E2E Test Suite**

```rust
// tests/e2e_invite_workflow.rs
#[tokio::test]
async fn test_full_invite_workflow() {
    // 1. User A creates event via bot
    // 2. Invites user B (Gmail) via /invite
    // 3. Check outbox has "calendar_invite" message
    // 4. Worker sends email with VEVENT
    // 5. Parse email reply (RSVP ACCEPTED)
    // 6. Verify attendee status updated in DB
    // 7. Check organizer receives notification
}
```

### **6.3 Documentation**

- [ ] Update README with invite examples
- [ ] Create INVITE_SYSTEM.md architecture doc
- [ ] Add CalDAV compliance matrix (which RFCs are implemented)
- [ ] Create user guide for Gmail/Outlook users receiving invites

---

## 📊 **Progress Tracking**

| Phase | Status | Completion | ETA |
|-------|--------|------------|-----|
| Phase 1: Core Features | ✅ Complete | 100% | Done |
| Phase 2: Invite System | 🔴 Not Started | 0% | 2-3 weeks |
| Phase 3: Incoming Invites | 🔴 Not Started | 0% | 2 weeks |
| Phase 4: Production Hardening | 🟡 Partial (20%) | 20% | 1 week |
| Phase 5: UX Enhancements | 🔴 Not Started | 0% | 1 week |
| Phase 6: Testing & Docs | 🟡 Partial (30%) | 30% | 1 week |

**Total remaining effort**: ~6-8 weeks to production-ready

---

## 🚀 **Definition of "Production Ready"**

The app is production-ready when:

- [x] Telegram bot creates/manages events
- [x] CalDAV sync works with Thunderbird/Apple Calendar
- [ ] **Users can invite Gmail/Outlook contacts** (email with VEVENT)
- [ ] **Users receive invites from Gmail/Outlook** (parse incoming emails)
- [ ] **RSVP workflow** (accept/decline, sends METHOD:REPLY emails)
- [ ] Rate limiting protects API
- [ ] Deleted events tracked for proper sync
- [ ] Integration tests pass
- [ ] Documentation complete

---

## 🎯 **Success Criteria**

**User can:**
1. Create event via Telegram bot: `/create Team Meeting, tomorrow 3pm, 1 hour`
2. Invite external users: `/invite <event_id> john@gmail.com sarah@outlook.com`
3. External users receive email with "Add to Calendar" .ics attachment
4. External users click Accept → Televent receives RSVP email → updates attendee status
5. User checks attendee list: `/attendees <event_id>` shows "John (Accepted), Sarah (Pending)"
6. Event syncs to Thunderbird via CalDAV with attendee list visible
7. External user updates event in Gmail Calendar → change email sent → Televent processes update

---

## 🧩 **Dependency Analysis**

**What can be done in parallel:**
- Phase 2.4 (iCalendar email) + Phase 4.1 (rate limiting) → No dependencies
- Phase 2.5 (outbox integration) → Requires Phase 2.2 (models) and 2.4 (mailer)
- Phase 3 (incoming invites) → Requires Phase 2 complete (can't process RSVPs without attendee table)

**Critical path**: Phase 2 → Phase 3 → Production

---

## 📞 **External Dependencies**

| Dependency | Purpose | Required When |
|------------|---------|---------------|
| Mailgun/SendGrid | Inbound email webhooks | Phase 3 |
| SMTP server | Outbound email | Already available (Mailpit for dev) |
| icalendar crate | Parse/generate VEVENT | Phase 2.4 |
| lettre multipart | Email attachments | Phase 2.4 |
| tower_governor | Rate limiting | Phase 4.1 |

---

## 🔄 **Future Enhancements (Post-Production)**

**Telegram Miniapp** (deferred until core features done):
- Inline calendar view with month/week/day views
- Visual event creation with datetime picker
- Drag-and-drop rescheduling
- Attendee management UI

**Other future work**:
- Recurring event exceptions (edit single instance)
- Event attachments (files, images)
- Video conferencing integration (Zoom/Meet links)
- SMS reminders (in addition to Telegram notifications)
- Team calendars (shared calendars for groups)

---

## 💡 **Key Insights**

**Current gap analysis**:
- ✅ **Personal calendar management**: Works perfectly
- ✅ **CalDAV desktop sync**: Works perfectly
- ❌ **Cross-platform invites**: Completely missing (0%)
- 🟡 **Production security**: Partial (auth works, rate limiting missing)

**Bottom line**: You have 60% of a personal calendar app, but 0% of the invite/RSVP system needed to collaborate with Gmail/Outlook users.

**Time to MVP**: 6-8 weeks of focused development to complete Phase 2-4.

**Recommendation**: Prioritize Phase 2 immediately. The invite system is the differentiator between "personal notes app" and "collaborative calendar platform."

---

## 📝 **Next Actions**

**For immediate start:**
1. Create `20260121_create_attendees_table.sql` migration
2. Add `EventAttendee` and enums to `crates/core/src/models.rs`
3. Implement `/invite` command in bot
4. Add iCalendar generation with ATTENDEE properties
5. Test email sending with .ics attachment using Mailpit

**Questions to resolve:**
- [ ] Do you want to use Mailgun/SendGrid webhooks or IMAP polling for incoming emails?
- [ ] Should user email be required on `/start` or optional?
- [ ] Should the bot ask for attendees during `/create` or require separate `/invite` command?

---

**Status**: Ready for Phase 2 implementation 🚀
