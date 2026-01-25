#!/usr/bin/env bash

# API URL
API_URL="http://localhost:3001/api/events"

# Auth Header (same as in frontend/src/lib/api.ts)
AUTH_HEADER="tma auth_date=1700000000&query_id=AAGyswdAAAAAAALLB0A&user=%7B%22id%22%3A123456789%2C%22first_name%22%3A%22Test%22%2C%22last_name%22%3A%22User%22%2C%22username%22%3A%22testuser%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Afalse%2C%22allows_write_to_pm%22%3Atrue%7D&hash=075e0d126e8e57060d9fdca6599f95482a4fdb97521e1a937f7c5dd8f6190719"

# Helper function to generate UUID
gen_uuid() {
    cat /proc/sys/kernel/random/uuid
}

# Helper to get date
get_date() {
    date -d "$1" +"%Y-%m-%dT%H:%M:%S"
}

echo "Seeding events..."

# Event 1: Team Sync (Tomorrow 10am)
START_DATE=$(get_date 'tomorrow 10:00')
END_DATE=$(get_date 'tomorrow 11:00')
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: $AUTH_HEADER" \
  -d "{
    \"uid\": \"$(gen_uuid)\",
    \"summary\": \"Team Sync\",
    \"description\": \"Weekly sync with the engineering team.\",
    \"location\": \"Google Meet\",
    \"start\": \"$START_DATE\",
    \"end\": \"$END_DATE\",
    \"is_all_day\": false,
    \"timezone\": \"UTC\"
  }"
echo ""

# Event 2: Lunch with Sarah (Tomorrow 1pm)
START_DATE=$(get_date 'tomorrow 13:00')
END_DATE=$(get_date 'tomorrow 14:30')
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: $AUTH_HEADER" \
  -d "{
    \"uid\": \"$(gen_uuid)\",
    \"summary\": \"Lunch with Sarah\",
    \"description\": \" discussing the new project.\",
    \"location\": \"Italian Place\",
    \"start\": \"$START_DATE\",
    \"end\": \"$END_DATE\",
    \"is_all_day\": false,
    \"timezone\": \"UTC\"
  }"
echo ""

# Event 3: Code Review (Day after tomorrow 3pm)
START_DATE=$(get_date '2 days 15:00')
END_DATE=$(get_date '2 days 16:00')
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: $AUTH_HEADER" \
  -d "{
    \"uid\": \"$(gen_uuid)\",
    \"summary\": \"Code Review: Validations\",
    \"description\": \"Reviewing PR #42 for input validation logic.\",
    \"location\": \"Office\",
    \"start\": \"$START_DATE\",
    \"end\": \"$END_DATE\",
    \"is_all_day\": false,
    \"timezone\": \"UTC\"
  }"
echo ""

# Event 4: Hackathon (Next Friday, All Day)
START_DATE=$(get_date 'next friday 09:00')
END_DATE=$(get_date 'next friday 18:00')
curl -X POST "$API_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: $AUTH_HEADER" \
  -d "{
    \"uid\": \"$(gen_uuid)\",
    \"summary\": \"Internal Hackathon\",
    \"description\": \"Building cool stuff!\",
    \"location\": \"HQ\",
    \"start\": \"$START_DATE\",
    \"end\": \"$END_DATE\",
    \"is_all_day\": true,
    \"timezone\": \"UTC\"
  }"
echo ""

echo "Done seeding events."
