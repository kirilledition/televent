'use client'

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api, EventResponse } from '@/lib/api'
import { format } from 'date-fns'
import { useRouter } from 'next/navigation'

export function EventList() {
  const {
    data: events,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['events'],
    queryFn: () => api.getEvents(),
  })

  const queryClient = useQueryClient()
  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteEvent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] })
    },
  })

  const handleDelete = (id: string) => {
    if (confirm('Are you sure you want to delete this event?')) {
      deleteMutation.mutate(id)
    }
  }

  if (isLoading) {
    return (
      <div className="text-subtext0 p-4 text-center">Loading events...</div>
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

  // Sort by start time
  const sortedEvents = [...events].sort(
    (a, b) => new Date(a.start).getTime() - new Date(b.start).getTime()
  )

  return (
    <div className="flex flex-col gap-3 p-4">
      {sortedEvents.map((event) => (
        <EventCard
          key={event.id}
          event={event}
          onDelete={() => handleDelete(event.id)}
        />
      ))}
    </div>
  )
}

function EventCard({
  event,
  onDelete,
}: {
  event: EventResponse
  onDelete: () => void
}) {
  const start = new Date(event.start)
  const end = new Date(event.end)
  const router = useRouter()

  return (
    <div className="bg-surface0 border-surface1 hover:bg-surface1/50 flex flex-col gap-2 rounded-xl border p-4 shadow-sm transition-all active:scale-[0.99]">
      <div className="flex items-start justify-between gap-2">
        <div>
          <h3 className="text-text line-clamp-1 font-semibold">
            {event.summary}
          </h3>
          <div className="text-subtext0 flex flex-wrap gap-x-3 text-sm">
            <span className="flex items-center gap-1">
              🕒 {format(start, 'MMM d, HH:mm')} - {format(end, 'HH:mm')}
            </span>
            {event.location && (
              <span className="line-clamp-1 flex items-center gap-1">
                📍 {event.location}
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="border-surface2 mt-2 flex justify-end gap-2 border-t pt-2">
        <button
          onClick={() => router.push(`/edit-event?id=${event.id}`)}
          className="bg-surface2 text-text hover:bg-overlay0 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
        >
          Edit
        </button>
        <button
          onClick={() => {
            console.log('Delete clicked for', event.id)
            onDelete()
          }}
          className="bg-surface2 text-red hover:bg-red/10 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
        >
          Delete
        </button>
      </div>
    </div>
  )
}
