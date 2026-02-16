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
  const startDate = new Date(apiEvent.start)
  const endDate = new Date(apiEvent.end)

  return {
    id: apiEvent.id,
    title: apiEvent.summary,
    // Format as YYYY-MM-DD for grouping
    date: format(startDate, 'yyyy-MM-dd'),
    // Format as HH:mm for display
    time: format(startDate, 'HH:mm'),
    duration: differenceInMinutes(endDate, startDate),
    location: apiEvent.location,
    description: apiEvent.description,
  }
}
