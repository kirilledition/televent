'use client'
import { Event } from '@/types/event'
import { Trash2, MapPin, Clock, ArrowLeft, AlignLeft } from 'lucide-react'
import { useRouter } from 'next/navigation'

interface EventDetailProps {
  event: Event
  onDelete: (id: string) => void
  onEdit: (event: Event) => void
}

export function EventDetail({ event, onDelete, onEdit }: EventDetailProps) {
  const router = useRouter()

  const formatDuration = (minutes: number) => {
    const hours = Math.floor(minutes / 60)
    const mins = minutes % 60
    let result = ''
    if (hours > 0) {
      result += `${hours}h`
      if (mins > 0) result += ` ${mins}m`
    } else if (mins > 0) {
      result = `${mins}m`
    }
    return result
  }

  return (
    <div
      className="min-h-screen"
      style={{ backgroundColor: 'var(--ctp-base)' }}
    >
      {/* Header */}
      <div
        className="sticky top-0 z-10 flex items-center justify-between px-4 py-4"
        style={{
          backgroundColor: 'var(--ctp-base)',
          borderBottom: '1px solid var(--ctp-surface0)',
        }}
      >
        <button
          onClick={() => router.back()}
          className="-ml-2 rounded-full p-2 transition-colors hover:bg-[var(--ctp-surface0)]"
          style={{ color: 'var(--ctp-text)' }}
        >
          <ArrowLeft className="h-6 w-6" />
        </button>
        <button
          onClick={() => onEdit(event)}
          className="rounded px-2 py-1 font-medium transition-colors hover:bg-[var(--ctp-surface0)]"
          style={{ color: 'var(--ctp-sapphire)' }}
        >
          Edit
        </button>
      </div>

      <div className="space-y-6 p-5">
        {/* Title */}
        <h1
          className="text-2xl font-semibold"
          style={{ color: 'var(--ctp-text)' }}
        >
          {event.title}
        </h1>

        {/* Time */}
        {(event.date || event.time) && (
          <div className="flex items-start gap-3">
            <Clock
              className="mt-0.5 h-5 w-5"
              style={{ color: 'var(--ctp-mauve)' }}
            />
            <div>
              <div className="font-medium" style={{ color: 'var(--ctp-text)' }}>
                {new Date(event.date + 'T00:00:00').toLocaleDateString(
                  undefined,
                  { weekday: 'long', month: 'long', day: 'numeric' }
                )}
                {event.time && ` at ${event.time}`}
              </div>
              {event.duration && (
                <div
                  className="text-sm"
                  style={{ color: 'var(--ctp-subtext0)' }}
                >
                  {formatDuration(event.duration)} duration
                </div>
              )}
            </div>
          </div>
        )}

        {/* Location */}
        {event.location && (
          <div className="flex items-start gap-3">
            <MapPin
              className="mt-0.5 h-5 w-5"
              style={{ color: 'var(--ctp-mauve)' }}
            />
            <div className="font-medium" style={{ color: 'var(--ctp-text)' }}>
              {event.location}
            </div>
          </div>
        )}

        {/* Description */}
        {event.description && (
          <div className="flex items-start gap-3">
            <AlignLeft
              className="mt-0.5 h-5 w-5"
              style={{ color: 'var(--ctp-mauve)' }}
            />
            <div
              className="whitespace-pre-wrap"
              style={{ color: 'var(--ctp-text)' }}
            >
              {event.description}
            </div>
          </div>
        )}

        {/* Delete Button */}
        <button
          onClick={() => onDelete(event.id)}
          className="mt-8 flex w-full items-center justify-center gap-2 rounded-lg py-3 font-medium transition-opacity hover:opacity-90"
          style={{
            backgroundColor: 'var(--ctp-surface0)',
            color: 'var(--ctp-red)',
          }}
        >
          <Trash2 className="h-5 w-5" />
          <span>Delete Event</span>
        </button>
      </div>
    </div>
  )
}
