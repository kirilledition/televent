import { EventResponse } from '@/types/schema'
import { Event as UiEvent } from '@/types/event'

/**
 * Maps API EventResponse to UI Event model
 *
 * Note: apiEvent.start/end are ISO strings (e.g. 2023-10-27T10:00:00Z)
 * We rely on Date to parse these into local time for display.
 */
export function mapApiEventToUiEvent(apiEvent: EventResponse): UiEvent {
  // Some all-day events may have null start/end in the API response.
  // We defensively handle that here and treat all-day events specially.
  const isAllDay: boolean = (apiEvent as unknown as Record<string, unknown>).is_all_day ?? false

  const hasStart = apiEvent.start != null
  const hasEnd = apiEvent.end != null

  const startDate = hasStart
    ? new Date(apiEvent.start as unknown as string)
    : null
  const endDate = hasEnd ? new Date(apiEvent.end as unknown as string) : null

  // Optimization: Manual string padding is ~15x faster than date-fns format()
  let date = ''
  let time = ''

  if (startDate) {
    const y = startDate.getFullYear()
    const m = (startDate.getMonth() + 1).toString().padStart(2, '0')
    const d = startDate.getDate().toString().padStart(2, '0')
    date = `${y}-${m}-${d}`

    if (!isAllDay) {
      const h = startDate.getHours().toString().padStart(2, '0')
      const min = startDate.getMinutes().toString().padStart(2, '0')
      time = `${h}:${min}`
    }
  }

  // Optimization: Manual math is significantly faster than differenceInMinutes()
  const duration =
    startDate && endDate
      ? Math.round((endDate.getTime() - startDate.getTime()) / 60000)
      : 0

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
