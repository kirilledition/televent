import { describe, it, expect } from 'vitest'
import {
  groupEventsByDate,
  groupEventsByDateEntries,
} from './groupEventsByDate'
import type { EventResponse } from './api'

// Mock event helper
const createEvent = (partial: Partial<EventResponse>): EventResponse => ({
  id: '1',
  uid: '1',
  user_id: 'u1',
  summary: 'Event',
  start_date: '2023-10-01',
  end_date: '2023-10-01',
  is_all_day: false,
  timezone: 'UTC',
  version: 1,
  etag: '1',
  created_at: '2023-09-01T00:00:00Z',
  updated_at: '2023-09-01T00:00:00Z',
  status: 'Confirmed' as any,
  ...partial,
})

describe('groupEventsByDate', () => {
  it('groups events by date', () => {
    const events = [
      createEvent({ id: '1', start_date: '2023-10-01' }),
      createEvent({ id: '2', start_date: '2023-10-01' }),
      createEvent({ id: '3', start_date: '2023-10-02' }),
    ]

    const grouped = groupEventsByDate(events)
    expect(Object.keys(grouped).length).toBe(2)
  })

  it('sorts events within groups by time', () => {
    const events = [
      createEvent({
        id: '2',
        summary: 'Event 2',
        start: '2023-10-01T10:00:00Z',
      }),
      createEvent({ id: '1', summary: 'Event 1', start: '2023-10-01T00:00:00Z' }), // Midnight (00:00)
      createEvent({
        id: '3',
        summary: 'Event 3',
        start: '2023-10-01T09:00:00Z',
      }),
    ]

    const grouped = groupEventsByDate(events)
    // Find the group (key depends on locale, so iterate values)
    const group = Object.values(grouped)[0]

    expect(group).toHaveLength(3)
    // Event 1 (midnight) -> Event 3 (9am) -> Event 2 (10am)
    expect(group[0].summary).toBe('Event 1')
    expect(group[1].summary).toBe('Event 3')
    expect(group[2].summary).toBe('Event 2')
  })
})

describe('groupEventsByDateEntries', () => {
  it('returns sorted entries by date', () => {
    const events = [
      createEvent({ id: '3', start_date: '2023-10-03' }),
      createEvent({ id: '1', start_date: '2023-10-01' }),
      createEvent({ id: '2', start_date: '2023-10-02' }),
    ]

    const entries = groupEventsByDateEntries(events)
    expect(entries).toHaveLength(3)
    // Check order
    // entries[0] should be 2023-10-01
    // entries[1] should be 2023-10-02
    // entries[2] should be 2023-10-03

    // We can verify by checking IDs since we know mapping
    expect(entries[0][1][0].id).toBe('1')
    expect(entries[1][1][0].id).toBe('2')
    expect(entries[2][1][0].id).toBe('3')
  })
})
