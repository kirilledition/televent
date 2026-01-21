#!/bin/bash
# Script to create test events via API

set -e

API_URL="http://localhost:3000/api/events"
CALENDAR_ID="b0beec28-36ee-4146-9bf2-9d0ebe1d6a18"

echo "Creating test events..."
echo ""

echo "1. Creating 'Team Meeting' for today (2026-01-21 14:00)..."
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "calendar_id": "'"$CALENDAR_ID"'",
    "uid": "test-meeting-1@televent.local",
    "summary": "Team Meeting",
    "description": "Discuss Q1 roadmap",
    "location": "Conference Room A",
    "start": "2026-01-21T14:00:00Z",
    "end": "2026-01-21T15:00:00Z",
    "is_all_day": false,
    "timezone": "UTC",
    "rrule": null
  }'
echo ""
echo ""

echo "2. Creating 'Coffee Chat' for tomorrow (2026-01-22 10:00)..."
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "calendar_id": "'"$CALENDAR_ID"'",
    "uid": "test-coffee-2@televent.local",
    "summary": "Coffee Chat",
    "description": "Catch up with team",
    "location": "Cafe Downtown",
    "start": "2026-01-22T10:00:00Z",
    "end": "2026-01-22T11:00:00Z",
    "is_all_day": false,
    "timezone": "UTC",
    "rrule": null
  }'
echo ""
echo ""

echo "3. Creating 'Project Deadline' for 2026-01-23..."
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "calendar_id": "'"$CALENDAR_ID"'",
    "uid": "test-deadline-3@televent.local",
    "summary": "Project Deadline",
    "description": "Final submission",
    "location": null,
    "start": "2026-01-23T09:00:00Z",
    "end": "2026-01-23T10:00:00Z",
    "is_all_day": false,
    "timezone": "UTC",
    "rrule": null
  }'
echo ""
echo ""

echo "✅ Done! Created 3 test events."
echo ""
echo "Now test in Telegram:"
echo "  /today     - Should show 'Team Meeting'"
echo "  /tomorrow  - Should show 'Coffee Chat'"
echo "  /week      - Should show all 3 events"
echo ""
echo "Check database:"
echo "  sudo -u postgres psql -d televent -c \"SELECT summary, start FROM events ORDER BY start;\""
