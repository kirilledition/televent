'use client'

import { useState, useEffect, useCallback } from 'react'
import {
  EventResponse,
  CreateEventRequest,
  UpdateEventRequest,
} from '@/types/schema'
import { api } from '@/lib/api'
import { EventList } from './_components/EventList'
import { CreateEvent } from './_components/CreateEvent'
import { Plus } from 'lucide-react'

export default function CalendarPage() {
  const [events, setEvents] = useState<EventResponse[]>([])
  const [isCreateOpen, setIsCreateOpen] = useState(false)
  const [editingEvent, setEditingEvent] = useState<EventResponse | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  // Load events
  useEffect(() => {
    loadEvents()
  }, [])

  const loadEvents = async () => {
    try {
      setIsLoading(true)
      const data = await api.getEvents()
      setEvents(data)
    } catch (error) {
      console.error('Error loading events:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const handleCreateEvent = useCallback(async (request: CreateEventRequest) => {
    try {
      const newEvent = await api.createEvent(request)
      setEvents((prev) => [...prev, newEvent])
      setIsCreateOpen(false)
    } catch (error) {
      console.error('Error creating event:', error)
    }
  }, [])

  // Memoized to keep prop stable for EventList -> EventItem
  const handleDeleteEvent = useCallback(async (id: string) => {
    try {
      await api.deleteEvent(id)
      setEvents((prev) => prev.filter((e) => e.id !== id))
    } catch (error) {
      console.error('Error deleting event:', error)
    }
  }, [])

  const handleUpdateEvent = useCallback(
    async (request: CreateEventRequest) => {
      if (!editingEvent) return

      try {
        // Map CreateEventRequest to UpdateEventRequest (filtering out non-updateable fields like uid/timezone if not supported)
        const updateData: UpdateEventRequest = {
          summary: request.summary,
          description: request.description,
          location: request.location,
          start: request.start,
          end: request.end,
          is_all_day: request.is_all_day,
          rrule: request.rrule,
        }

        const updated = await api.updateEvent(editingEvent.id, updateData)
        setEvents((prev) =>
          prev.map((e) => (e.id === editingEvent.id ? updated : e))
        )
        setEditingEvent(null)
      } catch (error) {
        console.error('Error updating event:', error)
      }
    },
    [editingEvent]
  )

  const handleCreateEventClick = useCallback(() => {
    setIsCreateOpen(true)
  }, [])

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
          onClick={handleCreateEventClick}
          className="mb-6 flex w-full items-center justify-center gap-2 rounded-lg px-5 py-3 font-medium shadow-sm transition-opacity hover:opacity-90"
          style={{
            backgroundColor: 'var(--ctp-mauve)',
            color: 'var(--ctp-crust)',
          }}
        >
          <Plus className="h-5 w-5" />
          <span>New event</span>
        </button>

        {/* Event List */}
        {isLoading ? (
          <div
            className="py-16 text-center"
            style={{ color: 'var(--ctp-overlay0)' }}
          >
            <p>Loading events...</p>
          </div>
        ) : (
          <EventList
            events={events}
            onDeleteEvent={handleDeleteEvent}
            onEditEvent={setEditingEvent}
            onCreateEvent={handleCreateEventClick}
          />
        )}

        {/* Create Event Modal */}
        {isCreateOpen && (
          <CreateEvent
            onClose={() => setIsCreateOpen(false)}
            onCreate={handleCreateEvent}
          />
        )}

        {/* Edit Event Modal */}
        {editingEvent && (
          <CreateEvent
            initialEvent={editingEvent}
            onClose={() => setEditingEvent(null)}
            onCreate={handleUpdateEvent}
          />
        )}
      </div>
    </div>
  )
}
