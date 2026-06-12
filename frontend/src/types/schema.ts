// Generated from backend/docs/openapi.json.

// Run `just gen-types` to refresh.

export type Timezone = string

export type UserId = string

export interface CreateDeviceRequest {
  name: string
}

export type EventTimingRequest =
  | {
      end: string
      kind: 'timed'
      start: string
      timezone: Timezone
    }
  | {
      end_date: string
      kind: 'all_day'
      start_date: string
    }

export interface CreateEventRequest {
  description?: string | null
  location?: string | null
  rrule?: string | null
  summary: string
  timing: EventTimingRequest
  uid: string
}

export interface DeviceListItem {
  created_at: string
  id: string
  last_used_at?: string | null
  name: string
}

export interface DevicePasswordResponse {
  created_at: string
  id: string
  last_used_at?: string | null
  name: string
  password?: string | null
}

export enum EventStatus {
  Confirmed = 'Confirmed',
  Tentative = 'Tentative',
  Cancelled = 'Cancelled',
}

export interface EventResponse {
  description?: string | null
  end?: string | null
  end_date?: string | null
  id: string
  is_all_day: boolean
  location?: string | null
  rrule?: string | null
  start?: string | null
  start_date?: string | null
  status: EventStatus
  summary: string
  timezone: Timezone
  uid: string
}

export interface ListEventsQuery {
  end?: string | null
  limit?: number | null
  offset?: number | null
  start?: string | null
}

export interface UpdateEventRequest {
  description?: string | null
  location?: string | null
  rrule?: string | null
  status?: null | EventStatus
  summary?: string | null
  timing?: null | EventTimingRequest
}

export interface MeResponse {
  authenticated: boolean
  id: UserId
  timezone: Timezone
  username?: string | null
}

export interface CalendarInfo {
  color: string
  id: UserId
  name: string
}

export type Event = EventResponse
