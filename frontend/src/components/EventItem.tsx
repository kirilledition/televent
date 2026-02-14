import { type MouseEvent, type KeyboardEvent } from 'react'
import { Event } from '@/types/event'
import { Trash2, MapPin, Clock } from 'lucide-react'

interface EventItemProps {
  event: Event
  onDelete: (id: string) => void
  onEdit: (event: Event) => void
}

export function EventItem({ event, onDelete, onEdit }: EventItemProps) {
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

  const handleDelete = (e: MouseEvent) => {
    e.stopPropagation()
    if (window.confirm('Are you sure you want to delete this event?')) {
      onDelete(event.id)
    }
  }

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onEdit(event)
    }
  }

  return (
    <div
      className="group relative mb-2 flex items-start gap-3 rounded-lg px-4 py-4 transition-colors hover:opacity-90"
      style={{ backgroundColor: 'var(--ctp-mantle)' }}
    >
      {/* Sapphire indicator */}
      <div
        className="absolute top-0 bottom-0 left-0 h-full w-1 rounded-full"
        style={{ backgroundColor: 'var(--ctp-sapphire)' }}
      />

      {/* Event content - Main interactive area */}
      <div
        className="min-w-0 flex-1 cursor-pointer rounded-md pr-12 pl-2 focus-visible:ring-2 focus-visible:ring-[var(--ctp-mauve)] focus-visible:outline-none"
        onClick={() => onEdit(event)}
        role="button"
        tabIndex={0}
        onKeyDown={handleKeyDown}
        aria-label={`Edit event: ${event.title}`}
      >
        <div className="mb-2 flex items-start gap-3">
          <h3
            className="text-lg font-medium"
            style={{ color: 'var(--ctp-text)' }}
          >
            {event.title}
          </h3>
        </div>

        <div className="space-y-1">
          {event.time && (
            <div
              className="flex items-center gap-2 text-sm"
              style={{ color: 'var(--ctp-subtext1)' }}
            >
              <Clock className="h-4 w-4" />
              <span>{event.time}</span>
              {event.duration && (
                <span style={{ color: 'var(--ctp-overlay1)' }}>
                  • {formatDuration(event.duration)}
                </span>
              )}
            </div>
          )}
          {event.location && (
            <div
              className="flex items-center gap-2 text-sm"
              style={{ color: 'var(--ctp-subtext1)' }}
            >
              <MapPin className="h-4 w-4" />
              <span>{event.location}</span>
            </div>
          )}
        </div>
      </div>

      {/* Delete button - Positioned absolutely to separate from main action */}
      <button
        onClick={handleDelete}
        className="absolute top-4 right-4 rounded-lg p-2 transition-all hover:opacity-70 focus-visible:ring-2 focus-visible:ring-[var(--ctp-red)] focus-visible:outline-none"
        style={{ backgroundColor: 'var(--ctp-surface0)' }}
        aria-label={`Delete event: ${event.title}`}
        title="Delete event"
      >
        <Trash2 className="h-4 w-4" style={{ color: 'var(--ctp-subtext0)' }} />
      </button>
    </div>
  )
}
