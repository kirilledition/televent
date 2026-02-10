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

      {/* Event content */}
      <div
        className="min-w-0 flex-1 cursor-pointer pl-2"
        onClick={() => onEdit(event)}
      >
        <div className="mb-2 flex items-start justify-between gap-3">
          <h3
            className="text-lg font-medium"
            style={{ color: 'var(--ctp-text)' }}
          >
            {event.title}
          </h3>

          {/* Delete button - always visible on mobile */}
          <button
            onClick={(e) => {
              e.stopPropagation()
              onDelete(event.id)
            }}
            className="rounded-lg p-2 transition-all hover:opacity-70"
            style={{ backgroundColor: 'var(--ctp-surface0)' }}
            aria-label="Delete event"
          >
            <Trash2
              className="h-4 w-4"
              style={{ color: 'var(--ctp-subtext0)' }}
            />
          </button>
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
    </div>
  )
}
