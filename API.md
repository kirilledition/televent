# api


## Version: 0.1.0

**Contact information:**  
kirill denisov  

**License:** MIT

### /calendars

#### GET
##### Summary:

List user's calendars

##### Description:

Returns a list containing the single user calendar.
Since user = calendar, this returns calendar properties from the user record.

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | List of calendars |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### /devices

#### GET
##### Summary:

List all device passwords for a user

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | List of device passwords |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

#### POST
##### Summary:

Create a new device password

##### Responses

| Code | Description |
| ---- | ----------- |
| 201 | Device password created |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### /devices/{device_id}

#### DELETE
##### Summary:

Delete a device password

##### Parameters

| Name | Located in | Description | Required | Schema |
| ---- | ---------- | ----------- | -------- | ---- |
| device_id | path | Device ID | Yes | string (uuid) |

##### Responses

| Code | Description |
| ---- | ----------- |
| 204 | Device password deleted successfully |
| 401 | Unauthorized |
| 404 | Device password not found |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### /events

#### GET
##### Summary:

List events

##### Parameters

| Name | Located in | Description | Required | Schema |
| ---- | ---------- | ----------- | -------- | ---- |
| start | path | Filter events starting after this time | Yes | string,null (date-time) |
| end | path | Filter events ending before this time | Yes | string,null (date-time) |
| limit | path | Maximum number of events to return | Yes | integer,null (int64) |
| offset | path | Number of events to skip | Yes | integer,null (int64) |

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | List of events |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

#### POST
##### Summary:

Create a new event

##### Responses

| Code | Description |
| ---- | ----------- |
| 201 | Event created successfully |
| 400 | Invalid request |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### /events/{id}

#### GET
##### Summary:

Get event by ID

##### Parameters

| Name | Located in | Description | Required | Schema |
| ---- | ---------- | ----------- | -------- | ---- |
| id | path | Event ID | Yes | string (uuid) |

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | Event details |
| 401 | Unauthorized |
| 404 | Event not found |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

#### PUT
##### Summary:

Update event

##### Parameters

| Name | Located in | Description | Required | Schema |
| ---- | ---------- | ----------- | -------- | ---- |
| id | path | Event ID | Yes | string (uuid) |

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | Event updated successfully |
| 400 | Invalid request |
| 401 | Unauthorized |
| 404 | Event not found |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

#### DELETE
##### Summary:

Delete event

##### Parameters

| Name | Located in | Description | Required | Schema |
| ---- | ---------- | ----------- | -------- | ---- |
| id | path | Event ID | Yes | string (uuid) |

##### Responses

| Code | Description |
| ---- | ----------- |
| 201 | Event deleted successfully |
| 401 | Unauthorized |
| 404 | Event not found |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### /health

#### GET
##### Summary:

Health check endpoint

##### Description:

Returns 200 OK if the server and database are healthy

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | Server is healthy |
| 503 | Server is degraded |

### /me

#### GET
##### Summary:

Get current user profile

##### Description:

Returns the authenticated user's profile based on Telegram initData.

##### Responses

| Code | Description |
| ---- | ----------- |
| 200 | Current user profile |
| 401 | Unauthorized |

##### Security

| Security Schema | Scopes |
| --- | --- |
| telegram_auth | |

### Models


#### AttendeeRole

Attendee role (RFC 5545 ROLE parameter)

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| AttendeeRole | string | Attendee role (RFC 5545 ROLE parameter) |  |

#### CalendarInfo

Calendar response (subset of User relevant to calendar functionality)

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| color | string |  | Yes |
| id | string |  | Yes |
| name | string |  | Yes |
| sync_token | string |  | Yes |

#### CreateDeviceRequest

Request to create a new device password

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| name | string | Device name/label (e.g., "iPhone", "Desktop") | Yes |

#### CreateEventRequest

Create event request

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| description | string,null | Detailed description | No |
| end | dateTime | End time (for timed events) | Yes |
| is_all_day | boolean | Whether this is an all-day event | Yes |
| location | string,null | Event location | No |
| rrule | string,null | RFC 5545 recurrence rule | No |
| start | dateTime | Start time (for timed events) | Yes |
| summary | string | Event summary/title | Yes |
| timezone | string | IANA timezone name | Yes |
| uid | string | iCalendar UID (stable across syncs) | Yes |

#### DeviceListItem

Device password list item (without password)

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| created_at | string |  | Yes |
| id | string (uuid) |  | Yes |
| last_used_at | string,null |  | No |
| name | string |  | Yes |

#### DevicePasswordResponse

Response containing generated device password

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| created_at | string |  | Yes |
| id | string (uuid) |  | Yes |
| last_used_at | string,null |  | No |
| name | string |  | Yes |
| password | string,null | Plain text password - only shown once at creation | No |

#### Event

Event entity

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| created_at | dateTime |  | Yes |
| description | string,null |  | No |
| end | string,null (date-time) |  | No |
| end_date | string,null (date) |  | No |
| etag | string |  | Yes |
| id | string (uuid) |  | Yes |
| is_all_day | boolean |  | Yes |
| location | string,null |  | No |
| rrule | string,null |  | No |
| start | string,null (date-time) |  | No |
| start_date | string,null (date) |  | No |
| status | [EventStatus](#eventstatus) |  | Yes |
| summary | string |  | Yes |
| timezone | [Timezone](#timezone) |  | Yes |
| uid | string |  | Yes |
| updated_at | dateTime |  | Yes |
| user_id | string | Owner's user ID (telegram_id) | Yes |
| version | integer |  | Yes |

#### EventAttendee

Event attendee with RSVP status

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| created_at | dateTime |  | Yes |
| email | string |  | Yes |
| event_id | string (uuid) |  | Yes |
| id | string (uuid) |  | Yes |
| role | [AttendeeRole](#attendeerole) |  | Yes |
| status | [ParticipationStatus](#participationstatus) |  | Yes |
| telegram_id | integer,null (int64) |  | No |
| updated_at | dateTime |  | Yes |

#### EventResponse

Event response (same as Event model)

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| created_at | dateTime |  | Yes |
| description | string,null |  | No |
| end | string,null (date-time) |  | No |
| end_date | string,null |  | No |
| etag | string |  | Yes |
| id | string (uuid) |  | Yes |
| is_all_day | boolean |  | Yes |
| location | string,null |  | No |
| rrule | string,null |  | No |
| start | string,null (date-time) |  | No |
| start_date | string,null |  | No |
| status | [EventStatus](#eventstatus) |  | Yes |
| summary | string |  | Yes |
| timezone | string |  | Yes |
| uid | string |  | Yes |
| updated_at | dateTime |  | Yes |
| user_id | string |  | Yes |
| version | integer |  | Yes |

#### EventStatus

Event status enumeration

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| EventStatus | string | Event status enumeration |  |

#### HealthResponse

Health check response

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| database | string | Database status ("healthy" or "unhealthy") | Yes |
| status | string | Server status ("ok" or "degraded") | Yes |

#### ListEventsQuery

List events query parameters

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| end | string,null (date-time) | Filter events ending before this time | No |
| limit | integer,null (int64) | Maximum number of events to return | No |
| offset | integer,null (int64) | Number of events to skip | No |
| start | string,null (date-time) | Filter events starting after this time | No |

#### MeResponse

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| authenticated | boolean |  | Yes |
| id | string |  | Yes |
| timezone | string |  | Yes |
| username | string,null |  | No |

#### ParticipationStatus

Participation status (RFC 5545 PARTSTAT parameter)

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| ParticipationStatus | string | Participation status (RFC 5545 PARTSTAT parameter) |  |

#### Timezone

Timezone newtype wrapping chrono_tz::Tz with SQLx and Serde support

Stored in database as TEXT (IANA timezone name like "America/New_York")

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| Timezone | string | Timezone newtype wrapping chrono_tz::Tz with SQLx and Serde support  Stored in database as TEXT (IANA timezone name like "America/New_York") |  |

#### UpdateEventRequest

Update event request

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| description | string,null |  | No |
| end | string,null (date-time) |  | No |
| is_all_day | boolean,null |  | No |
| location | string,null |  | No |
| rrule | string,null |  | No |
| start | string,null (date-time) |  | No |
| status |  |  | No |
| summary | string,null |  | No |

#### User

User entity (includes calendar data since user = calendar)

The telegram_id serves as the primary key and unique identifier.
Calendar properties are merged into this struct since each user has exactly one calendar.

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| calendar_color | string | Calendar hex color for UI | Yes |
| calendar_name | string | Calendar display name | Yes |
| created_at | dateTime |  | Yes |
| ctag | string | Collection tag for change detection | Yes |
| id | string | Primary key: Telegram's permanent numeric ID | Yes |
| sync_token | string | RFC 6578 sync token for CalDAV sync-collection | Yes |
| telegram_username | string,null | Telegram username/handle (can change, used for CalDAV URLs) | No |
| timezone | [Timezone](#timezone) | IANA timezone (e.g., "Asia/Singapore") | Yes |
| updated_at | dateTime |  | Yes |

#### UserId

User ID newtype wrapping Telegram's permanent numeric ID

This serves as the primary identifier for both users and their calendars,
since each user has exactly one calendar.

| Name | Type | Description | Required |
| ---- | ---- | ----------- | -------- |
| UserId | string | User ID newtype wrapping Telegram's permanent numeric ID  This serves as the primary identifier for both users and their calendars, since each user has exactly one calendar. |  |