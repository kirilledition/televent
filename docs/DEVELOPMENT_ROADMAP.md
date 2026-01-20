# **Televent Development Roadmap \- Path to Production**

Last Updated: 2026-01-20  
Status: Phase 1 Complete, Entering Phase 2 (Interceptor)

## **📋 Project Vision**

Create a **Telegram-first calendar** that:

1. ✅ Manages events via Telegram bot  
2. ✅ Syncs to desktop calendar apps (Thunderbird, Apple Calendar) via CalDAV  
3. 🟡 **Invites other Televent users** (Interim Step: Interceptor Pattern)  
4. ❌ **Invites Gmail/Outlook users** (Future Step: SMTP Integration)  
5. ❌ **Receives invites** from Gmail/Outlook users (Future Step: Email Parsing)  
6. ⏳ Telegram miniapp with calendar UI (Deferred until core logic is solid)

## **🎯 Current State Assessment**

### **✅ What's Working (Phase 1 \- COMPLETE)**

| Feature | Status | Notes |
| :---- | :---- | :---- |
| Telegram bot | ✅ Working | 11 commands implemented |
| CalDAV server | ✅ Working | RFC 4791 compliant, read/write |
| Device passwords | ✅ Working | Argon2id auth for CalDAV clients |
| Event CRUD | ✅ Working | Create, read, update, delete events |
| Background worker | ✅ Working | Outbox pattern implemented |
| PostgreSQL schema | ✅ Working | Proper indexes, triggers, constraints |

### **❌ Critical Gaps (Blocks MVP)**

| Feature | Status | Impact | Priority |
| :---- | :---- | :---- | :---- |
| **Interceptor Invite System** | ❌ Missing | Can't invite other users internal to Televent | **P0** |
| **Attendee DB Schema** | ❌ Missing | No RSVP tracking tables | **P0** |
| **Supabase Validation** | ❌ Pending | Production DB environment not tested | **P0** |
| **Railway Deployment** | ❌ Pending | No live URL for manual QA | **P0** |

## **🚧 Development Phases**

## **Phase 2: The Interceptor (Internal Invites) 🔴 IMMEDIATE PRIORITY**

**Goal**: Build the full invite logic (attendees, RSVPs, notifications) but "fake" the email transport layer to avoid paid SMTP dependencies for now.  
**Duration**: 1-2 weeks

### **2.1 Database Schema for Attendees**

* **Migration**: Create event\_attendees table.  
* **Columns**: event\_id, email (can be fake: tg\_123@televent.internal), status (pending/accepted/declined), telegram\_id (nullable).  
* **Validation**: Ensure uniqueness constraints (one RSVP per email per event).

### **2.2 Core Logic: The "Interceptor"**

* **Internal Emails**: Implement logic in crates/core to generate internal email addresses: tg\_\<telegram\_id\>@televent.internal.  
* **Worker Processor**: Modify crates/worker to check recipient domains.  
  * **If @televent.internal:** Do **not** send SMTP. Instead, send a Telegram message via the Bot API: "📅 You have been invited\!".  
  * **If External:** Log "External invite skipped (MVP Mode)" and mark as done.

### **2.3 Bot RSVP Commands**

* **Commands**: Implement /rsvp \<event\_id\> \<status\> (accept/decline).  
* **Flow**:  
  1. User A creates event /new Meeting.  
  2. User A invites User B (/invite @UserB).  
  3. User B gets Telegram message (via Interceptor).  
  4. User B clicks /rsvp accept.  
  5. User A gets notification "User B accepted".

## **Phase 3: Staging & QA (Supabase) 🟡 CRITICAL**

**Goal**: Validate the entire stack on a production-like database (Supabase) before spending money on Railway.  
**Duration**: 1 week

### crap i personally want to fix when everything working
- dont use tg_ prefix for internal adresses
- when local send logs to jsonl file
- when deployed send logs to logs management
- add descriptions to commands inside bot
- add template message for event creation
- better names for commands
- name televent lowercase everywhere
- versioning rules
- minimize emojis in docs
- concatenate docs to one readme

### **3.1 Supabase Setup**

* **Action**: Create a free Supabase project.  
* **Config**: Add DATABASE\_URL from Supabase to local .env.  
* **Migration**: Run sqlx migrate run against Supabase.  
* **Constraint Check**: Verify triggers and RLS (if any) work on Supabase Postgres.

### **3.2 Local-to-Remote Testing**

* **Action**: Run api, bot, and worker locally (Docker Compose or bare metal) but connected to **Supabase**.  
* **Scenario Test**:  
  1. Create event.  
  2. Sync with Thunderbird (local IP).  
  3. Invite another Telegram user (Interceptor).  
  4. Verify data persistence in Supabase dashboard.

## **Phase 4: Production Deployment (Railway) 🟢 GO-LIVE**

**Goal**: A live, accessible URL for the MVP.  
Duration: 1 week  
Pre-requisite: Phase 3 passed 100%.

### **4.1 Railway Configuration**

* **Action**: Connect GitHub repo to Railway.  
* **Services**: Deploy api, bot, and worker as separate services.  
* **Env Vars**: Set DATABASE\_URL (Supabase), TELEGRAM\_TOKEN, RUST\_LOG.

### **4.2 Manual QA (The "Hand Test")**

* **Connectivity**: Verify https://televent.up.railway.app/caldav works from a mobile phone (4G/5G, not local Wi-Fi).  
* **Bot Responsiveness**: Ensure webhooks/polling work without lag.  
* **Worker Reliability**: Check logs to ensure the "Interceptor" is firing correctly in the cloud environment.

### **4.3 GDPR**

* **GDPR Complience**: All base principles of GDPR must be implemented


## **Phase 5: Visual Interface (Mini App) 🔵 FUTURE**

**Goal**: Add the Dioxus-based calendar grid.  
**Status**: Deferred until Phase 4 is stable.

* **UI**: Monthly grid view using Tailwind CSS.  
* **Auth**: Telegram initData validation.  
* **Hosting**: Vercel (Frontend) \+ Railway (API).

## **Phase 6: The "Un-Mocking" (External Invites) 🟣 FUTURE**

**Goal**: Enable real SMTP for Gmail/Outlook users.  
**Status**: Deferred until Phase 4 is stable.

* **SMTP**: Integrate Postmark/SendGrid.  
* **Logic**: Update Interceptor to pass non-internal emails to the SMTP transport.  
* **Inbound**: Implement inbound email parsing for replies.
