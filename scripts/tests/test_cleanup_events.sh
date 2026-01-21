#!/bin/bash
# Script to clean up test events

set -e

echo "Cleaning up test events..."

sudo -u postgres psql -d televent << 'EOF'
DELETE FROM events
WHERE uid LIKE 'test-%@televent.local';

SELECT 'Deleted ' || count(*) || ' test events' as result
FROM events
WHERE uid LIKE 'test-%@televent.local';
EOF

echo ""
echo "✅ Test events cleaned up!"
echo ""
echo "Verify in Telegram with /today, /tomorrow, /week"
