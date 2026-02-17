import { describe, it, expect } from 'vitest'
import { mapApiEventToUiEvent } from './mappers'
import { EventResponse, EventStatus } from '@/types/schema'

// Mock helper
const createApiEvent = (partial: Partial<EventResponse>): EventResponse => ({
  id: '1',
  user_id: 'u1',
  uid: 'uid-1',
  summary: 'Test Event',
  status: EventStatus.Confirmed,
  timezone: 'UTC',
  version: 1,
  etag: 'tag',
  created_at: '2023-01-01T00:00:00Z',
  updated_at: '2023-01-01T00:00:00Z',
  start: '2023-10-27T10:05:00Z', // ISO string
  end: '2023-10-27T11:05:00Z',
  start_date: '2023-10-27',
  end_date: '2023-10-27',
  is_all_day: false,
  ...partial,
})

describe('mapApiEventToUiEvent', () => {
  it('maps standard event correctly', () => {
    // Note: This test depends on the local timezone of the runner.
    // We can't easily mock the timezone in Node/Vitest without extra setup.
    // So we will verify the structure and that it produces *valid* strings.

    const apiEvent = createApiEvent({
      start: '2023-10-27T10:05:00Z',
      end: '2023-10-27T11:05:00Z',
    })

    const uiEvent = mapApiEventToUiEvent(apiEvent)

    expect(uiEvent.id).toBe('1')
    expect(uiEvent.title).toBe('Test Event')

    // Check date format YYYY-MM-DD
    expect(uiEvent.date).toMatch(/^\d{4}-\d{2}-\d{2}$/)

    // Check time format HH:mm
    expect(uiEvent.time).toMatch(/^\d{2}:\d{2}$/)

    // Check duration (60 minutes)
    expect(uiEvent.duration).toBe(60)
  })

  it('maps all-day event correctly', () => {
    const apiEvent = createApiEvent({
      is_all_day: true,
      start: '2023-10-27T00:00:00Z', // API might return start even for all-day
      end: '2023-10-28T00:00:00Z',
    })

    const uiEvent = mapApiEventToUiEvent(apiEvent)

    expect(uiEvent.date).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    expect(uiEvent.time).toBe('') // All-day events have empty time
  })

  it('handles events across midnight/days', () => {
    const apiEvent = createApiEvent({
      start: '2023-10-27T23:30:00Z',
      end: '2023-10-28T00:30:00Z',
    })

    const uiEvent = mapApiEventToUiEvent(apiEvent)
    expect(uiEvent.duration).toBe(60)
  })

  it('handles missing end time', () => {
    const apiEvent = createApiEvent({
      start: '2023-10-27T10:00:00Z',
      end: undefined as unknown as string, // Simulate missing end
    })

    const uiEvent = mapApiEventToUiEvent(apiEvent)
    expect(uiEvent.duration).toBe(0)
  })

  it('verifies exact local time mapping (mocking timezone via date construction)', () => {
    // To verify exact mapping without relying on system timezone,
    // we can check if the output matches what a local Date object produces.

    const startIso = '2023-01-15T08:30:00Z'
    const endIso = '2023-01-15T09:45:00Z'

    const apiEvent = createApiEvent({
      start: startIso,
      end: endIso,
    })

    const uiEvent = mapApiEventToUiEvent(apiEvent)

    const d = new Date(startIso)
    const expectedYear = d.getFullYear()
    const expectedMonth = String(d.getMonth() + 1).padStart(2, '0')
    const expectedDay = String(d.getDate()).padStart(2, '0')
    const expectedDate = `${expectedYear}-${expectedMonth}-${expectedDay}`

    const expectedHour = String(d.getHours()).padStart(2, '0')
    const expectedMinute = String(d.getMinutes()).padStart(2, '0')
    const expectedTime = `${expectedHour}:${expectedMinute}`

    expect(uiEvent.date).toBe(expectedDate)
    expect(uiEvent.time).toBe(expectedTime)
    expect(uiEvent.duration).toBe(75)
  })
})
