import type { Event } from './api'

/**
 * Get the event's start date/time for sorting and grouping.
 * Handles both timed events (start) and all-day events (start_date).
 */
function getEventDateTime(event: Event): Date {
  if (event.start) {
    return new Date(event.start)
  } else if (event.start_date) {
    return new Date(event.start_date)
  }
  // Fallback to current time if neither is set (shouldn't happen with valid data)
  return new Date()
}

// Reuse formatter to avoid repeated Intl.DateTimeFormat instantiation (expensive)
// Note: This uses the system locale at the time of module loading.
const dateFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: 'long',
  year: 'numeric',
  month: 'long',
  day: 'numeric',
})

export function groupEventsByDate(events: Event[]): Record<string, Event[]> {
  // Group by date string, caching the timestamp for sorting
  const grouped: Record<string, { event: Event; time: number }[]> = {}

  events.forEach((event) => {
    const date = getEventDateTime(event)
    const time = date.getTime()
    const dateKey = dateFormatter.format(date)

    if (!grouped[dateKey]) {
      grouped[dateKey] = []
    }
    grouped[dateKey].push({ event, time })
  })

  // Sort events within each group by time using the cached timestamp
  // Then map back to the original Event object
  const result: Record<string, Event[]> = {}

  Object.entries(grouped).forEach(([dateKey, group]) => {
    group.sort((a, b) => a.time - b.time)
    result[dateKey] = group.map((item) => item.event)
  })

  return result
}

export function groupEventsByDateEntries(events: Event[]): [string, Event[]][] {
  const grouped = groupEventsByDate(events)
  // Sort entries by the first event's date in each group
  // grouped values are already sorted by time, so index 0 is the earliest
  return Object.entries(grouped).sort((a, b) => {
    return (
      getEventDateTime(a[1][0]).getTime() - getEventDateTime(b[1][0]).getTime()
    )
  })
}
