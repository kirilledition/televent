import { EventResponse } from '@/types/schema'
import { Event as UiEvent } from '@/types/event'
import { format, differenceInMinutes } from 'date-fns'

/**
 * Maps API EventResponse to UI Event model
 *
 * Note: apiEvent.start/end are ISO strings (e.g. 2023-10-27T10:00:00Z)
 * We rely on date-fns/Date to parse these into local time for display.
 */
export function mapApiEventToUiEvent(apiEvent: EventResponse): UiEvent {
  // Some all-day events may have null start/end in the API response.
  // We defensively handle that here and treat all-day events specially.
  const isAllDay: boolean = (apiEvent as any).is_all_day ?? false

  const hasStart = apiEvent.start != null
  const hasEnd = apiEvent.end != null

  const startDate = hasStart ? new Date(apiEvent.start as unknown as string) : null
  const endDate = hasEnd ? new Date(apiEvent.end as unknown as string) : null

  const date = startDate ? format(startDate, 'yyyy-MM-dd') : ''
  // For all-day events (or missing start), we omit the specific time.
  const time = !startDate || isAllDay ? '' : format(startDate, 'HH:mm')
  const duration =
    startDate && endDate ? differenceInMinutes(endDate, startDate) : 0

  return {
    id: apiEvent.id,
    title: apiEvent.summary,
    // Format as YYYY-MM-DD for grouping
    date,
    // Format as HH:mm for display (empty for all-day events)
    time,
    duration,
    location: apiEvent.location,
    description: apiEvent.description,
  }
}
