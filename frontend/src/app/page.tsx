'use client'

import { useCallback, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { EventList } from '../components/EventList'
import { api } from '@/lib/api'
import { mapApiEventToUiEvent } from '@/lib/mappers'
import { Plus, Loader2 } from 'lucide-react'
import { useRouter } from 'next/navigation'
import { Event } from '@/types/event'

export default function CalendarPage() {
  const router = useRouter()
  const queryClient = useQueryClient()

  const {
    data: eventsData,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['events'],
    queryFn: () => api.getEvents(),
  })

  const { mutate: deleteEvent } = useMutation({
    mutationFn: (id: string) => api.deleteEvent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] })
    },
  })

  const handleDeleteEvent = useCallback(
    (id: string) => {
      deleteEvent(id)
    },
    [deleteEvent]
  )

  const handleEditEvent = useCallback(
    (event: Event) => {
      router.push(`/event-detail?id=${event.id}`)
    },
    [router]
  )

  // Optimization: Memoize the events array to prevent unnecessary re-renders in EventList.
  // Without this, mapApiEventToUiEvent runs on every render, creating a new array reference,
  // which causes EventList's internal useMemo (for sorting/grouping) to re-run,
  // and breaks React.memo optimization in EventItem.
  const events = useMemo(
    () => (eventsData ? eventsData.map(mapApiEventToUiEvent) : []),
    [eventsData]
  )

  if (isLoading) {
    return (
      <div
        className="flex h-screen items-center justify-center"
        style={{ backgroundColor: 'var(--ctp-base)' }}
      >
        <Loader2
          className="h-8 w-8 animate-spin"
          style={{ color: 'var(--ctp-mauve)' }}
        />
      </div>
    )
  }

  if (error) {
    return (
      <div
        className="flex h-screen items-center justify-center p-4 text-center"
        style={{ backgroundColor: 'var(--ctp-base)', color: 'var(--ctp-red)' }}
      >
        Error loading events: {error.message}
      </div>
    )
  }

  return (
    <div
      className="min-h-screen"
      style={{ backgroundColor: 'var(--ctp-base)' }}
    >
      <div className="mx-auto max-w-2xl px-4 py-8">
        {/* Header */}
        <div className="mb-6">
          <h1
            className="mb-1 text-3xl font-semibold"
            style={{ color: 'var(--ctp-text)' }}
          >
            Calendar
          </h1>
          <p className="text-sm" style={{ color: 'var(--ctp-subtext0)' }}>
            Keep track of your events
          </p>
        </div>

        {/* New Event Button */}
        <button
          onClick={() => router.push('/create')}
          className="mb-6 flex w-full items-center justify-center gap-2 rounded-lg px-5 py-3 font-medium shadow-sm transition-opacity hover:opacity-90 focus-visible:ring-2 focus-visible:ring-[var(--ctp-mauve)] focus-visible:outline-none"
          style={{
            backgroundColor: 'var(--ctp-mauve)',
            color: 'var(--ctp-crust)',
          }}
        >
          <Plus className="h-5 w-5" />
          <span>New event</span>
        </button>

        {/* Event List */}
        <EventList
          events={events}
          onDeleteEvent={handleDeleteEvent}
          onEditEvent={handleEditEvent}
        />
      </div>
    </div>
  )
}
