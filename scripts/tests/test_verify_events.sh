#!/bin/bash
# Script to verify events in database

echo "=== Events in Database ==="
echo ""

sudo -u postgres psql -d televent << 'EOF'
SELECT
    summary,
    to_char(start, 'YYYY-MM-DD HH24:MI') as start_time,
    to_char("end", 'YYYY-MM-DD HH24:MI') as end_time,
    location,
    status
FROM events e
JOIN calendars c ON e.calendar_id = c.id
JOIN users u ON c.user_id = u.id
WHERE u.telegram_id = 185819179
ORDER BY start;
EOF

echo ""
echo "=== Event Count ==="
sudo -u postgres psql -d televent -t -c "
SELECT count(*) as total_events
FROM events e
JOIN calendars c ON e.calendar_id = c.id
JOIN users u ON c.user_id = u.id
WHERE u.telegram_id = 185819179;"

echo ""
echo "Now test in Telegram:"
echo "  /today     - Show today's events"
echo "  /tomorrow  - Show tomorrow's events"
echo "  /week      - Show this week's events"
