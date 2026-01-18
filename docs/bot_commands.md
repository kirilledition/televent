# Telegram Bot Commands

## User Commands

### Account Setup
- `/start` - Initialize account and show setup instructions
- `/device add <name>` - Generate CalDAV password for a device
- `/device list` - Show all device passwords
- `/device revoke <id>` - Delete a device password

### Event Management
- `/create` - Create event (guided conversation flow)
- `/today` - List today's events
- `/tomorrow` - List tomorrow's events
- `/week` - List this week's events
- `/cancel <event_id>` - Cancel an event

### GDPR & Privacy
- `/export` - Request GDPR data export
- `/delete_account` - Initiate account deletion (30-day grace period)

## Command Details

### /create - Event Creation Flow

Interactive conversation to create events:

```
User: /create
Bot: "Event title?"
User: "Team standup"
Bot: "When? (e.g., 'tomorrow 10am' or '2026-01-20 14:00')"
User: "tomorrow 10am"
Bot: "Duration? (e.g., '30m', '1h')"
User: "30m"
Bot: "Description? (or /skip)"
User: "Weekly sync meeting"
Bot: ✅ "Event created: Team standup, Jan 19 10:00-10:30"
```

### Natural Language Date Parsing

Supported formats:
- `tomorrow 10am`
- `next monday 14:00`
- `2026-01-25 14:00`
- `in 2 hours`
- `at 3pm`

### Duration Format

- `30m` - 30 minutes
- `1h` - 1 hour
- `1h30m` - 1 hour 30 minutes
- `90m` - 90 minutes

## Future Commands (Not Implemented Yet)

- `/settings` - Update user preferences
- `/timezone` - Change timezone
- `/help` - Show command help
