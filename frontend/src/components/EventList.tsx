import { useMemo } from 'react'
import { Event } from '@/types/event'
import { EventItem } from './EventItem'

interface EventListProps {
  events: Event[]
  onDeleteEvent: (id: string) => void
  onEditEvent: (event: Event) => void
}

export function EventList({
  events,
  onDeleteEvent,
  onEditEvent,
}: EventListProps) {
  // Sort events by date and time
  // Optimization: useMemo to prevent re-sorting on every render
  // Optimization: String comparison is ~20x faster than new Date() parsing
  const sortedEvents = useMemo(() => {
    return [...events].sort((a, b) => {
      // Primary sort by date
      if (a.date < b.date) return -1
      if (a.date > b.date) return 1

      // Secondary sort by time (treat missing time as 00:00)
      const timeA = a.time || '00:00'
      const timeB = b.time || '00:00'
      if (timeA < timeB) return -1
      if (timeA > timeB) return 1

      return 0
    })
  }, [events])

  // Group events by date
  // Optimization: useMemo to prevent re-grouping on every render
  const groupedEvents = useMemo(() => {
    return sortedEvents.reduce(
      (acc, event) => {
        const date = event.date
        if (!acc[date]) {
          acc[date] = []
        }
        acc[date].push(event)
        return acc
      },
      {} as Record<string, Event[]>
    )
  }, [sortedEvents])

  // Optimization: Calculate reference dates once per render instead of per row.
  // We avoid useMemo here to ensure dates update correctly if the component re-renders across midnight.
  const today = new Date()
  today.setHours(0, 0, 0, 0)

  const tomorrow = new Date(today)
  tomorrow.setDate(tomorrow.getDate() + 1)

  const formatDateHeader = (dateStr: string) => {
    const date = new Date(dateStr + 'T00:00:00')

    if (date.getTime() === today.getTime()) {
      return 'Today'
    } else if (date.getTime() === tomorrow.getTime()) {
      return 'Tomorrow'
    } else {
      return date.toLocaleDateString('en-US', {
        weekday: 'long',
        month: 'long',
        day: 'numeric',
        year:
          date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
      })
    }
  }

  if (sortedEvents.length === 0) {
    return (
      <div
        className="py-16 text-center"
        style={{ color: 'var(--ctp-overlay0)' }}
      >
        <p className="text-sm">
          No events yet. Create your first event to get started.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {Object.entries(groupedEvents).map(([date, dateEvents]) => (
        <div key={date}>
          <div
            className="mb-3 px-2 text-sm font-medium"
            style={{ color: 'var(--ctp-subtext0)' }}
          >
            {formatDateHeader(date)}
          </div>
          <div className="space-y-0">
            {dateEvents.map((event) => (
              <EventItem
                key={event.id}
                event={event}
                onDelete={onDeleteEvent}
                onEdit={onEditEvent}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}
