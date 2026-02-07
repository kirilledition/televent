'use client'

import { useState, memo, useRef, useEffect } from 'react'
import { EventResponse } from '@/types/schema'
import { Trash2, MapPin, Clock, Check, X } from 'lucide-react'
import { format, differenceInMinutes, parseISO } from 'date-fns'

interface EventItemProps {
  event: EventResponse
  onDelete: (id: string) => void
  onEdit: (event: EventResponse) => void
}

// Memoized to prevent re-renders when other items change or parent re-renders
export const EventItem = memo(function EventItem({
  event,
  onDelete,
  onEdit,
}: EventItemProps) {
  const [isConfirming, setIsConfirming] = useState(false)
  const deleteBtnRef = useRef<HTMLButtonElement>(null)
  const cancelBtnRef = useRef<HTMLButtonElement>(null)
  const prevIsConfirming = useRef(isConfirming)

  // Focus management for destructive actions
  useEffect(() => {
    if (prevIsConfirming.current !== isConfirming) {
      if (isConfirming) {
        // When entering confirmation mode, focus cancel button for safety
        cancelBtnRef.current?.focus()
      } else {
        // When cancelling, focus back to delete button
        deleteBtnRef.current?.focus()
      }
      prevIsConfirming.current = isConfirming
    }
  }, [isConfirming])

  // Handle all-day events where start/end might be null but start_date/end_date exist
  const start = parseISO(event.start || event.start_date)
  const end = parseISO(event.end || event.end_date)
  const duration = differenceInMinutes(end, start)

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

      {/* Event content wrapper */}
      <div className="flex min-w-0 flex-1 items-start gap-3 pl-2">
        {/* Clickable event details */}
        <div
          className="min-w-0 flex-1 cursor-pointer rounded-sm focus-visible:ring-2 focus-visible:ring-[var(--ctp-sapphire)] focus-visible:outline-none"
          onClick={() => onEdit(event)}
          role="button"
          tabIndex={0}
          aria-label={`Edit event: ${event.summary}`}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              onEdit(event)
            }
          }}
        >
          <div className="mb-2">
            <h3
              className="text-lg font-medium"
              style={{ color: 'var(--ctp-text)' }}
            >
              {event.summary}
            </h3>
          </div>

          <div className="space-y-1">
            {!event.is_all_day ? (
              <div
                className="flex items-center gap-2 text-sm"
                style={{ color: 'var(--ctp-subtext1)' }}
              >
                <Clock className="h-4 w-4" />
                <span>{format(start, 'HH:mm')}</span>
                {duration > 0 && (
                  <span style={{ color: 'var(--ctp-overlay1)' }}>
                    • {formatDuration(duration)}
                  </span>
                )}
              </div>
            ) : (
              <div
                className="flex items-center gap-2 text-sm"
                style={{ color: 'var(--ctp-subtext1)' }}
              >
                <Clock className="h-4 w-4" />
                <span>All Day</span>
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

        {/* Delete button - always visible on mobile */}
        <div className="flex-shrink-0">
          {isConfirming ? (
            <div className="flex gap-2">
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onDelete(event.id)
                }}
                className="rounded-lg p-2 transition-all hover:opacity-70"
                style={{ backgroundColor: 'var(--ctp-surface0)' }}
                aria-label="Confirm delete"
              >
                <Check
                  className="h-4 w-4"
                  style={{ color: 'var(--ctp-red)' }}
                />
              </button>
              <button
                ref={cancelBtnRef}
                onClick={(e) => {
                  e.stopPropagation()
                  setIsConfirming(false)
                }}
                className="rounded-lg p-2 transition-all hover:opacity-70"
                style={{ backgroundColor: 'var(--ctp-surface0)' }}
                aria-label="Cancel delete"
              >
                <X
                  className="h-4 w-4"
                  style={{ color: 'var(--ctp-subtext0)' }}
                />
              </button>
            </div>
          ) : (
            <button
              ref={deleteBtnRef}
              onClick={(e) => {
                e.stopPropagation()
                setIsConfirming(true)
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
          )}
        </div>
      </div>
    </div>
  )
})
