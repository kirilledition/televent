#!/bin/bash
# Integration test using cadaver CLI client

set -e

# Configuration (matching test_thunderbird_caldav.sh)
USER_ID="ffa2daf0-4492-40b3-8ba4-795f4854c6ac"
USERNAME="185819179"
PASSWORD="DNNBp6gCIt04A4yi"
BASE_URL="http://localhost:3000/caldav/$USER_ID/"
RC_FILE=$(mktemp)

# Create a temporary cadaver config file for non-interactive auth
cat <<EOF > "$RC_FILE"
machine localhost
    login $USERNAME
    password $PASSWORD
EOF

cleanup() {
    rm -f "$RC_FILE"
}
trap cleanup EXIT

echo "=== Cadaver Integration Test ==="
echo "Target: $BASE_URL"

# Create a temporary event file for testing
EVENT_FILE=$(mktemp --suffix=.ics)
cat <<EOF > "$EVENT_FILE"
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Televent//Test//EN
BEGIN:VEVENT
UID:cadaver-test-$(date +%s)
DTSTAMP:20240101T000000Z
DTSTART:20260125T100000Z
DURATION:PT1H
SUMMARY:Cadaver Test Event
DESCRIPTION:Created via cadaver integration test
END:VEVENT
END:VCALENDAR
EOF

echo "1. Testing connection and LS..."
# We pipe commands to cadaver. 
# 'ls' lists collection contents.
echo "ls" | cadaver --rcfile="$RC_FILE" "$BASE_URL"

echo "2. Testing PUT (Upload event)..."
# Upload the test event
echo "put $EVENT_FILE test-event.ics" | cadaver --rcfile="$RC_FILE" "$BASE_URL"

echo "3. Testing GET (Download event)..."
# Download checking it exists
echo "get test-event.ics -" | cadaver --rcfile="$RC_FILE" "$BASE_URL"

echo "4. Testing DELETE..."
# Delete the event
echo "delete test-event.ics" | cadaver --rcfile="$RC_FILE" "$BASE_URL"

echo "=== Test Complete ==="
