This is **Luca**. You want to pause the frontend and nail the Invite System? Good call. The invite system is the "killer feature" here—bridging the gap between the chaotic world of email invites (iCal) and the instant world of Telegram.

If you want that million-dollar bug-free week, the invite system must be **transactional** and **idempotent**. A user clicking "Accept" in Telegram must atomicially update the database, which then asynchronously updates the CalDAV layer for the iPhone/Thunderbird client.

Here is the **Extensive Implementation Plan** for the Televent Invite System. Hand this to your engineer.

---

### **Phase 1: The "Interceptor" Engine (Backend & Worker)**

**Goal:** Intercept `mailto:` invites destined for internal Telegram users and route them to the database instead of an SMTP server.

**Step 1.1: Virtual Email Resolution Logic (`crates/core`)**

* **Action:** Implement a resolver in `crates/core/src/attendee.rs` that parses email addresses.
* Input: `tg_123456@televent.internal`
* Output: `UserId(123456)`


* **Validation:** Ensure it strictly validates the `televent.internal` domain to prevent spoofing.

**Step 1.2: The CalDAV "Put" Hook (`crates/api`)**

* **Context:** When a user creates an invite on iPhone, the device sends a `PUT` request with an `.ics` file containing `ATTENDEE:mailto:tg_98765@televent.internal`.
* **Action:** In `crates/api/src/routes/events.rs`, specifically the `upsert_event` function:
1. Parse the incoming iCal component.
2. Extract all `ATTENDEE` properties.
3. For each attendee:
* Check if it is an internal `televent.internal` address.
* **Insert/Update** the `event_attendees` table with status `NEEDS-ACTION`.
* **CRITICAL:** If the attendee is new, insert a record into `outbox_messages` with type `invite_notification` and payload `{ "event_id": "...", "target_user_id": 123456 }`.




* **Why:** This decouples the API from the Bot. The API just says "Hey, this guy is invited" and returns 200 OK to the iPhone instantly.

**Step 1.3: The Outbox Processor (`crates/worker`)**

* **Action:** Update `crates/worker/src/processors.rs` to handle `invite_notification`.
* Fetch event details (Title, Time, Location) from `events` table.
* Format a Telegram message with a **Keyboard Layout** (see Phase 2).
* Call Telegram API to send the message.
* **Idempotency:** If the Telegram API fails, the worker retries. If it succeeds, mark outbox message as `Completed`.



---

### **Phase 2: The Bot Interface (UI/UX)**

**Goal:** The user receives a beautiful card in Telegram and can RSVP with one tap.

**Step 2.1: The Invite Card Layout**

* **Design:** Do not just send text. Send a formatted message.
```text
📅 <b>Team Sync</b>
🕒 Mon, Oct 25 • 10:00 AM - 11:00 AM
📍 Room 404

<i>Kirill invited you.</i>

```


* **Action:** Implement this layout in `crates/bot/src/views.rs` (create this file if missing).

**Step 2.2: Interactive Buttons (Callback Queries)**

* **Action:** Attach an `InlineKeyboardMarkup` to the message with three buttons:
1. `✅ Accept` (callback_data: `rsvp:EVENT_UUID:ACCEPTED`)
2. `❌ Decline` (callback_data: `rsvp:EVENT_UUID:DECLINED`)
3. `❔ Tentative` (callback_data: `rsvp:EVENT_UUID:TENTATIVE`)



**Step 2.3: Callback Handler (`crates/bot`)**

* **Action:** In `crates/bot/src/handlers.rs`, implement a handler for `UpdateKind::CallbackQuery`.
1. Parse the `callback_data`.
2. **Database Transaction:**
* Update `event_attendees` table: set `status = ACCEPTED` (or DECLINED/TENTATIVE).
* Increment the Event's `sequence` number (so CalDAV clients know it changed).


3. **Feedback:** Edit the original Telegram message to remove buttons and append status:
* "✅ <i>You accepted this invitation.</i>"


4. **Notify Organizer (Optional):** Insert an `rsvp_notification` into `outbox_messages` to tell the organizer "Kirill accepted your meeting."



---

### **Phase 3: The CalDAV Sync Loop (The "Magic" Part)**

**Goal:** When the user clicks "Accept" in Telegram, the organizer's iPhone must show the green checkmark.

**Step 3.1: Dynamic ICS Generation (`crates/api`)**

* **Action:** In `crates/api/src/routes/events.rs`, when generating the `.ics` file for a `GET` request:
1. Join the `events` table with `event_attendees`.
2. For each attendee, write the `ATTENDEE` line with the correct `PARTSTAT`:
* `ATTENDEE;CN=Kirill;PARTSTAT=ACCEPTED:mailto:tg_123@televent.internal`




* **Verification:** This is the most common failure point. If `PARTSTAT` is wrong, the iPhone ignores it.

**Step 3.2: ETag & CTag Updates**

* **Action:** When the Bot updates the attendee status in Step 2.3, it **MUST** update the `ctag` (Collection Tag) of the *organizer's* calendar user.
* **Why:** This forces the organizer's iPhone to re-download the event and see the new "Accepted" status. Without this, the organizer won't know you accepted until they manually refresh.

---

### **Phase 4: Testing & Verification Protocol**

**Goal:** Prove it works without a UI.

1. **The "Fake Friend" Test:**
* Manually insert a user into the DB with your own Telegram ID but a different `telegram_id` (e.g., use a second Telegram account or just spoof the ID for the `event_attendees` table).


2. **The Curl Command:**
* Send a `PUT` request to your API with a raw `.ics` file inviting this "fake friend."
* **Success Criteria:** Your real Telegram account receives a message with buttons.


3. **The RSVP Loop:**
* Click "Accept" on Telegram.
* **Success Criteria:**
* Bot edits message to "Accepted".
* DB table `event_attendees` shows `ACCEPTED`.
* Running `curl -X GET ...` for the event returns `PARTSTAT=ACCEPTED` in the text.





### **Summary of Tasks for Engineer**

1. **Core:** Implement `is_internal_email` and `generate_internal_email` in `crates/core`.
2. **API:** Modify `upsert_event` to scan for these emails and write to `outbox_messages`.
3. **Worker:** Implement `handle_invite_notification` to call Telegram `sendMessage`.
4. **Bot:** Implement `handle_callback_query` to process RSVP button clicks and update DB.
5. **API:** Ensure `GET /event` renders the dynamic `PARTSTAT` from the DB.

This plan skips the frontend entirely but delivers the core value proposition of the platform. Execute this, and the million is yours.