'use client'

import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api, EventResponse } from '@/lib/api'
import { format, isToday, isTomorrow } from 'date-fns'
import Link from 'next/link'
import { ChevronRight, Loader2 } from 'lucide-react'

export function EventList() {
  const {
    data: events,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['events'],
    queryFn: () => api.getEvents(),
  })

  // Memoize sorting and grouping to prevent recalculation on every render.
  // Only recalculates when events array reference changes (after fetch/mutation).
  // Must be called unconditionally to satisfy React hooks rules.
  const groupedEvents = useMemo(() => {
    if (!events || events.length === 0) return {}
    const sorted = [...events].sort(
      (a, b) => new Date(a.start).getTime() - new Date(b.start).getTime()
    )
    return sorted.reduce(
      (groups, event) => {
        const date = new Date(event.start)
        const dateKey = format(date, 'yyyy-MM-dd')
        if (!groups[dateKey]) {
          groups[dateKey] = []
        }
        groups[dateKey].push(event)
        return groups
      },
      {} as Record<string, EventResponse[]>
    )
  }, [events])

  if (isLoading) {
    return (
      <div className="flex justify-center p-8">
        <Loader2 className="text-subtext0 h-8 w-8 animate-spin" />
        <span className="sr-only">Loading events...</span>
      </div>
    )
  }

  if (error) {
    return <div className="text-red p-4 text-center">Error loading events.</div>
  }

  if (!events || events.length === 0) {
    return (
      <div className="text-overlay2 flex flex-col items-center justify-center p-8">
        <p className="mb-2 text-lg">No events found</p>
        <p className="text-sm">Create one to get started!</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-0 pb-20">
      {Object.entries(groupedEvents).map(([dateKey, groupEvents]) => {
        const date = new Date(groupEvents[0].start)
        let title = format(date, 'MMMM d')
        if (isToday(date)) title = 'Today'
        else if (isTomorrow(date)) title = 'Tomorrow'

        return (
          <div key={dateKey} className="flex flex-col">
            <div className="text-subtext0 px-4 py-2 text-sm font-medium uppercase">
              {title}
            </div>
            <div className="bg-surface0 border-surface1 border-y">
              {groupEvents.map((event, index) => (
                <Link
                  key={event.id}
                  href={`/edit-event?id=${event.id}`}
                  className={`border-surface1 hover:bg-surface1/50 active:bg-surface1 flex cursor-pointer items-center justify-between px-4 py-3 transition-colors ${
                    index !== groupEvents.length - 1 ? 'border-b' : ''
                  }`}
                >
                  <div className="flex flex-col gap-0.5">
                    <div className="text-text font-medium">{event.summary}</div>
                    <div className="text-subtext0 text-sm">
                      {format(new Date(event.start), 'HH:mm')} -{' '}
                      {format(new Date(event.end), 'HH:mm')}
                    </div>
                  </div>
                  <ChevronRight className="text-subtext0 h-5 w-5" />
                </Link>
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}
