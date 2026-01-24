#!/bin/bash
# Integration test using curl (CLI client)
# Used as alternative to cadaver which proved difficult to script non-interactively

set -e

# Configuration
USER_ID="ffa2daf0-4492-40b3-8ba4-795f4854c6ac"
USERNAME="185819179"
PASSWORD="DNNBp6gCIt04A4yi"
BASE_URL="http://localhost:3000/caldav/$USER_ID/"
EVENT_UID="cli-test-$(date +%s)"

echo "=== CLI Integration Test (curl) ==="
echo "Target: $BASE_URL"

# Helper function to check last command success
check_status() {
    if [ $? -eq 0 ]; then
        echo "✅ Success"
    else
        echo "❌ Failed"
        exit 1
    fi
}

echo ""
echo "1. Testing PROPFIND (List items)..."
curl -s -X PROPFIND "$BASE_URL" \
    -u "$USERNAME:$PASSWORD" \
    -H "Depth: 1" \
    -d '<?xml version="1.0" encoding="utf-8" ?><d:propfind xmlns:d="DAV:"><d:prop><d:displayname/></d:prop></d:propfind>' \
    | grep -q "multistatus"
check_status

echo "2. Testing PUT (Create Event)..."
EVENT_DATA="BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Televent//Test//EN
BEGIN:VEVENT
UID:$EVENT_UID
DTSTAMP:20240101T000000Z
DTSTART:20260125T120000Z
DURATION:PT1H
SUMMARY:CLI Test Event
DESCRIPTION:Created via curl integration test
END:VEVENT
END:VCALENDAR"

# Need to compute ETag for match? New event doesn't need If-Match.
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL$EVENT_UID.ics" \
    -u "$USERNAME:$PASSWORD" \
    -H "Content-Type: text/calendar" \
    -d "$EVENT_DATA")

if [ "$HTTP_CODE" == "201" ] || [ "$HTTP_CODE" == "204" ]; then
    echo "✅ Success (HTTP $HTTP_CODE)"
else
    echo "❌ Failed (HTTP $HTTP_CODE)"
    exit 1
fi

echo "3. Testing GET (Fetch Event & Get ETag)..."
# Fetch event and extract ETag
RESPONSE=$(curl -s -i -X GET "$BASE_URL$EVENT_UID.ics" -u "$USERNAME:$PASSWORD")
ETAG=$(echo "$RESPONSE" | grep -i "ETag:" | awk '{print $2}' | tr -d '\r')
echo "Fetched ETag: $ETAG"

if [[ -z "$ETAG" ]]; then
    echo "❌ Failed to get ETag"
    exit 1
fi
echo "✅ Success"

echo "4. Testing PUT (Update Event)..."
# Just change the summary
UPDATE_DATA="${EVENT_DATA/SUMMARY:CLI Test Event/SUMMARY:CLI Test Event Updated}"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL$EVENT_UID.ics" \
    -u "$USERNAME:$PASSWORD" \
    -H "Content-Type: text/calendar" \
    -H "If-Match: $ETAG" \
    -d "$UPDATE_DATA")

if [ "$HTTP_CODE" == "204" ] || [ "$HTTP_CODE" == "200" ]; then
    echo "✅ Success (HTTP $HTTP_CODE)"
else
    echo "❌ Failed (HTTP $HTTP_CODE)"
    exit 1
fi

echo "5. Testing DELETE (Remove Event)..."
# We should technically use If-Match, but for test we force delete
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE_URL$EVENT_UID.ics" \
    -u "$USERNAME:$PASSWORD")

if [ "$HTTP_CODE" == "204" ]; then
    echo "✅ Success (HTTP 204)"
else
    echo "❌ Failed (HTTP $HTTP_CODE)"
    exit 1
fi

echo ""
echo "=== Test Complete ==="
