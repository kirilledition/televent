# Test Scripts

## Test Scripts (`scripts/tests/`)

### Event Management
- **`test_create_events.sh`** - Creates 3 test events via API
  - Team Meeting (today at 14:00)
  - Coffee Chat (tomorrow at 10:00)
  - Project Deadline (day after tomorrow at 09:00)

- **`test_verify_events.sh`** - Shows all events in database for user 185819179

- **`test_cleanup_events.sh`** - Deletes all test events (UIDs starting with `test-`)

### CalDAV Testing
- **`test_thunderbird_caldav.sh`** - Comprehensive CalDAV compatibility test
  - Tests OPTIONS, PROPFIND, REPORT
  - Verifies all required properties
  - Shows what Thunderbird sees

## Utility Scripts (`scripts/utils/`)

- **`get_caldav_url.sh`** - Displays correct CalDAV credentials for user 185819179

## Usage

### Testing Event Creation Flow
```bash
# 1. Create test events
./scripts/tests/test_create_events.sh

# 2. Verify they're in database
./scripts/tests/test_verify_events.sh

# 3. Test in Telegram bot
# /today, /tomorrow, /week

# 4. Clean up when done
./scripts/tests/test_cleanup_events.sh
```

### Testing CalDAV
```bash
# Get your credentials
./scripts/utils/get_caldav_url.sh

# Run compatibility test
./scripts/tests/test_thunderbird_caldav.sh
```

## Notes

All scripts assume:
- API server running on `localhost:3000`
- PostgreSQL running with database `televent`
- User telegram_id: `185819179`
- User UUID: `ffa2daf0-4492-40b3-8ba4-795f4854c6ac`

Update these values in the scripts if your setup differs.
