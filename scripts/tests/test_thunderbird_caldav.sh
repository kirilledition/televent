#!/bin/bash
# Comprehensive CalDAV test for Thunderbird compatibility

USER_ID="ffa2daf0-4492-40b3-8ba4-795f4854c6ac"
USERNAME="185819179"
PASSWORD="DNNBp6gCIt04A4yi"
BASE_URL="http://localhost:3000/caldav/$USER_ID/"

echo "=== Thunderbird CalDAV Compatibility Test ==="
echo ""
echo "Server URL: $BASE_URL"
echo ""

# Test 1: OPTIONS
echo "1. Testing OPTIONS (CalDAV capabilities)..."
curl -s -X OPTIONS "$BASE_URL" -u "$USERNAME:$PASSWORD" -i | grep -E "(HTTP/|DAV:|Allow:)"
echo ""

# Test 2: PROPFIND Depth 0 (calendar properties)
echo "2. Testing PROPFIND Depth:0 (calendar properties)..."
RESPONSE=$(curl -s -X PROPFIND "$BASE_URL" \
  -u "$USERNAME:$PASSWORD" \
  -H "Depth: 0" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8" ?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:displayname />
    <d:resourcetype />
    <c:calendar-description />
    <c:calendar-color />
    <c:supported-calendar-component-set />
    <c:getctag />
  </d:prop>
</d:propfind>')

echo "$RESPONSE" | xmllint --format - 2>/dev/null | head -30
echo ""

# Test 3: PROPFIND Depth 1 (with event list)
echo "3. Testing PROPFIND Depth:1 (list events)..."
DEPTH1=$(curl -s -X PROPFIND "$BASE_URL" \
  -u "$USERNAME:$PASSWORD" \
  -H "Depth: 1" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8" ?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:getetag />
    <d:getlastmodified />
  </d:prop>
</d:propfind>')

echo "Number of event resources: $(echo "$DEPTH1" | grep -c 'test-.*\.ics')"
echo "$DEPTH1" | grep -A2 "test-meeting" | head -10
echo ""

# Test 4: REPORT calendar-query (how Thunderbird fetches events)
echo "4. Testing REPORT calendar-query..."
REPORT=$(curl -s -X REPORT "$BASE_URL" \
  -u "$USERNAME:$PASSWORD" \
  -H "Depth: 1" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8" ?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT" />
    </c:comp-filter>
  </c:filter>
</c:calendar-query>')

EVENT_COUNT=$(echo "$REPORT" | grep -c "BEGIN:VEVENT")
echo "Events returned: $EVENT_COUNT"
echo "$REPORT" | grep "SUMMARY:" | head -3
echo ""

# Test 5: Check for required properties
echo "5. Checking required CalDAV properties..."
echo "   ✓ displayname: $(echo "$RESPONSE" | grep -c displayname)"
echo "   ✓ resourcetype (collection+calendar): $(echo "$RESPONSE" | grep -c 'cal:calendar')"
echo "   ✓ supported-calendar-component-set: $(echo "$RESPONSE" | grep -c 'supported-calendar-component-set')"
echo "   ✓ getctag: $(echo "$RESPONSE" | grep -c 'getctag')"
echo ""

echo "=== Summary ==="
echo "If all tests passed, the CalDAV server is Thunderbird-compatible."
echo ""
echo "To add in Thunderbird:"
echo "1. Calendar → New Calendar → On the Network"
echo "2. Choose CalDAV"
echo "3. Location: $BASE_URL"
echo "4. Username: $USERNAME"
echo "5. (Uncheck 'autodiscover' if asked)"
echo ""
echo "Common issues:"
echo "- If Thunderbird says 'unavailable', check API logs for errors"
echo "- Try restarting Thunderbird after adding the calendar"
echo "- Use 127.0.0.1 instead of localhost if connection fails"
