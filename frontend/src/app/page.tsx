'use client'

import { useState } from 'react'
import { EventList } from '../components/EventList'
import { DUMMY_EVENTS } from '@/lib/dummy-data'
import { Plus } from 'lucide-react'
import { useRouter } from 'next/navigation'

export default function CalendarPage() {
  const router = useRouter()
  const [events, setEvents] = useState(DUMMY_EVENTS)

  const handleDeleteEvent = (id: string) => {
    setEvents(events.filter((e) => e.id !== id))
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
        <EventList
          events={events}
          onDeleteEvent={handleDeleteEvent}
          onEditEvent={(event) => router.push(`/event/${event.id}`)}
        />
      </div>
    </div>
  )
}
