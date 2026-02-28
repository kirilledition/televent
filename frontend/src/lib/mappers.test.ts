import { describe, it, expect } from 'vitest'
import { mapApiEventToUiEvent } from './mappers'

describe('mapApiEventToUiEvent', () => {
  it('maps correctly for timed events', () => {
    const apiEvent = {
      id: '123',
      summary: 'Test Event',
      start: '2023-10-27T10:05:00Z',
      end: '2023-10-27T11:05:00Z',
      is_all_day: false,
      location: 'Test Loc',
      description: 'Test Desc',
      uid: '',
      version: 1,
      etag: '',
      status: 'Confirmed',
      timezone: 'UTC',
      created_at: '',
      updated_at: ''
    }

    // Convert to local time as Date would
    const s = new Date('2023-10-27T10:05:00Z');
    const y = s.getFullYear();
    const m = String(s.getMonth() + 1).padStart(2, '0');
    const d = String(s.getDate()).padStart(2, '0');
    const expectedDate = `${y}-${m}-${d}`;

    const h = String(s.getHours()).padStart(2, '0');
    const min = String(s.getMinutes()).padStart(2, '0');
    const expectedTime = `${h}:${min}`;

    const uiEvent = mapApiEventToUiEvent(apiEvent as unknown as EventResponse);

    expect(uiEvent.id).toBe('123');
    expect(uiEvent.title).toBe('Test Event');
    expect(uiEvent.date).toBe(expectedDate);
    expect(uiEvent.time).toBe(expectedTime);
    expect(uiEvent.duration).toBe(60);
  })

  it('maps correctly for all-day events', () => {
    const apiEvent = {
      id: '124',
      summary: 'All Day Event',
      start: '2023-10-27T00:00:00Z',
      end: '2023-10-28T00:00:00Z',
      is_all_day: true,
      location: '',
      description: '',
      uid: '',
      version: 1,
      etag: '',
      status: 'Confirmed',
      timezone: 'UTC',
      created_at: '',
      updated_at: ''
    }

    const s = new Date('2023-10-27T00:00:00Z');
    const y = s.getFullYear();
    const m = String(s.getMonth() + 1).padStart(2, '0');
    const d = String(s.getDate()).padStart(2, '0');
    const expectedDate = `${y}-${m}-${d}`;

    const uiEvent = mapApiEventToUiEvent(apiEvent as unknown as EventResponse);

    expect(uiEvent.date).toBe(expectedDate);
    expect(uiEvent.time).toBe('');
  })
})
