#!/bin/bash
# Get the correct CalDAV URL for your user

echo "=== Correct CalDAV URL ==="
echo ""

USER_ID=$(sudo -u postgres psql -d televent -t -c "
SELECT u.id
FROM users u
WHERE u.telegram_id = 185819179;" | tr -d ' ')

echo "📋 CalDAV Server URL:"
echo "   http://localhost:3000/caldav/$USER_ID/"
echo ""
echo "👤 Username: 185819179"
echo "🔑 Password: (from /device list in Telegram)"
echo ""
echo "Use this URL in your CalDAV client (Thunderbird, Evolution, etc.)"
