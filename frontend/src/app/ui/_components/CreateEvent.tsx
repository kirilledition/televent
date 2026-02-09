'use client'

import { useState, useEffect, useRef } from 'react'
import { EventResponse, CreateEventRequest } from '@/types/schema'
import { MapPin, Loader2 } from 'lucide-react'
import { parseISO, format, differenceInMinutes, addMinutes } from 'date-fns'

interface CreateEventProps {
  onClose: () => void
  onCreate: (request: CreateEventRequest) => Promise<void> | void
  initialEvent?: EventResponse
}

// Generate time options in 5-minute intervals
const generateTimeOptions = () => {
  const options = []
  for (let hour = 0; hour < 24; hour++) {
    for (let minute = 0; minute < 60; minute += 5) {
      const timeString = `${hour.toString().padStart(2, '0')}:${minute.toString().padStart(2, '0')}`
      options.push(timeString)
    }
  }
  return options
}

// Generate duration options
const generateDurationOptions = () => {
  const options = []
  for (let minutes = 15; minutes <= 480; minutes += 15) {
    const hours = Math.floor(minutes / 60)
    const mins = minutes % 60
    let label = ''
    if (hours > 0) {
      label += `${hours}h`
      if (mins > 0) label += ` ${mins}m`
    } else {
      label = `${mins}m`
    }
    options.push({ value: minutes, label })
  }
  return options
}

const timeOptions = generateTimeOptions()
const durationOptions = generateDurationOptions()

// Round time to nearest 5 minutes (round up)
const roundToNearest5Min = (date: Date) => {
  const minutes = date.getMinutes()
  const roundedMinutes = Math.ceil(minutes / 5) * 5
  const newDate = new Date(date)
  newDate.setMinutes(roundedMinutes)
  newDate.setSeconds(0)
  newDate.setMilliseconds(0)

  // Handle hour overflow
  if (roundedMinutes >= 60) {
    newDate.setMinutes(roundedMinutes % 60)
    newDate.setHours(newDate.getHours() + 1)
  }

  return newDate
}

export function CreateEvent({
  onClose,
  onCreate,
  initialEvent,
}: CreateEventProps) {
  const now = new Date()
  const roundedTime = roundToNearest5Min(now)

  // Initialize state from initialEvent or defaults
  const [title, setTitle] = useState(initialEvent?.summary || '')

  // Parse initial dates if available
  const initialStart = initialEvent ? parseISO(initialEvent.start) : null
  const initialEnd = initialEvent ? parseISO(initialEvent.end) : null

  const [date, setDate] = useState(
    initialStart
      ? format(initialStart, 'yyyy-MM-dd')
      : now.toISOString().split('T')[0]
  )
  const [time, setTime] = useState(
    initialStart
      ? format(initialStart, 'HH:mm')
      : `${roundedTime.getHours().toString().padStart(2, '0')}:${roundedTime.getMinutes().toString().padStart(2, '0')}`
  )

  const [duration, setDuration] = useState(
    initialStart && initialEnd
      ? differenceInMinutes(initialEnd, initialStart)
      : 45
  )

  const [location, setLocation] = useState(initialEvent?.location || '')
  const [showDatePicker, setShowDatePicker] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)

  const timePickerRef = useRef<HTMLDivElement>(null)
  const durationPickerRef = useRef<HTMLDivElement>(null)
  const isScrollingTimeRef = useRef(false)
  const isScrollingDurationRef = useRef(false)

  // Scroll to selected time on mount
  useEffect(() => {
    if (timePickerRef.current) {
      const selectedIndex = timeOptions.indexOf(time)
      if (selectedIndex >= 0) {
        const itemHeight = 48
        const scrollPosition = selectedIndex * itemHeight
        timePickerRef.current.scrollTop = scrollPosition
      }
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Scroll to selected duration on mount
  useEffect(() => {
    if (durationPickerRef.current) {
      const selectedIndex = durationOptions.findIndex(
        (d) => d.value === duration
      )
      if (selectedIndex >= 0) {
        const itemHeight = 48
        const scrollPosition = selectedIndex * itemHeight
        durationPickerRef.current.scrollTop = scrollPosition
      }
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Add Escape key listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()

    if (!title || !date || isSubmitting) {
      return
    }

    try {
      setIsSubmitting(true)
      // Construct start and end dates
      const startDateTime = new Date(`${date}T${time}:00`)
      const endDateTime = addMinutes(startDateTime, duration)

      // Construct request object
      const request: CreateEventRequest = {
        uid: initialEvent?.uid || crypto.randomUUID(), // Preserve UID on edit, new on create
        summary: title,
        start: startDateTime.toISOString(),
        end: endDateTime.toISOString(),
        is_all_day: false,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        location: location || undefined,
        // Handle optional fields
        description: initialEvent?.description,
        rrule: initialEvent?.rrule,
      }

      await onCreate(request)
    } finally {
      setIsSubmitting(false)
    }
  }

  // Generate calendar days for current month
  const generateCalendarDays = () => {
    const selectedDate = new Date(date + 'T00:00:00')
    const year = selectedDate.getFullYear()
    const month = selectedDate.getMonth()

    const firstDay = new Date(year, month, 1)
    const lastDay = new Date(year, month + 1, 0)
    const startingDayOfWeek = firstDay.getDay()

    const days = []

    // Add empty cells for days before month starts
    for (let i = 0; i < startingDayOfWeek; i++) {
      days.push(null)
    }

    // Add all days of the month
    for (let day = 1; day <= lastDay.getDate(); day++) {
      days.push(day)
    }

    return { days, month, year }
  }

  const { days, month, year } = generateCalendarDays()
  const monthNames = [
    'January',
    'February',
    'March',
    'April',
    'May',
    'June',
    'July',
    'August',
    'September',
    'October',
    'November',
    'December',
  ]

  const handleDateSelect = (day: number) => {
    const newDate = new Date(year, month, day)
    // Use local date string format YYYY-MM-DD
    // To safe avoid UTC shifts, we can just format it manually or use date-fns format
    // But since we constructed it with local year/month, it's local.
    // The previous implementation used toISOString().split('T')[0] which might be risky if close to midnight UTC.
    // Better:
    const yearStr = newDate.getFullYear()
    const monthStr = (newDate.getMonth() + 1).toString().padStart(2, '0')
    const dayStr = newDate.getDate().toString().padStart(2, '0')
    setDate(`${yearStr}-${monthStr}-${dayStr}`)

    setShowDatePicker(false)
  }

  // ... (keeping scroll logic same as original)
  const handleTimeScrollEnd = () => {
    if (!timePickerRef.current || isScrollingTimeRef.current) return

    const scrollTop = timePickerRef.current.scrollTop
    const itemHeight = 48
    const index = Math.round(scrollTop / itemHeight)
    const targetScroll = index * itemHeight

    timePickerRef.current.scrollTo({
      top: targetScroll,
      behavior: 'smooth',
    })

    if (timeOptions[index]) {
      setTime(timeOptions[index])
    }
  }

  const handleDurationScrollEnd = () => {
    if (!durationPickerRef.current || isScrollingDurationRef.current) return

    const scrollTop = durationPickerRef.current.scrollTop
    const itemHeight = 48
    const index = Math.round(scrollTop / itemHeight)
    const targetScroll = index * itemHeight

    durationPickerRef.current.scrollTo({
      top: targetScroll,
      behavior: 'smooth',
    })

    if (durationOptions[index]) {
      setDuration(durationOptions[index].value)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-end justify-center sm:items-center"
      style={{ backgroundColor: 'rgba(0, 0, 0, 0.5)' }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <div
        className="max-h-[90vh] w-full overflow-y-auto rounded-t-2xl sm:max-w-md sm:rounded-2xl"
        style={{ backgroundColor: 'var(--ctp-base)' }}
      >
        {/* Header */}
        <div
          className="sticky top-0 z-20 flex items-center justify-between px-5 py-4"
          style={{
            backgroundColor: 'var(--ctp-base)',
            borderBottom: '1px solid var(--ctp-surface0)',
          }}
        >
          <button
            type="button"
            onClick={onClose}
            className="font-medium disabled:cursor-not-allowed disabled:opacity-50"
            style={{ color: 'var(--ctp-sapphire)' }}
            disabled={isSubmitting}
          >
            Cancel
          </button>
          <h2
            id="modal-title"
            className="text-lg font-semibold"
            style={{ color: 'var(--ctp-text)' }}
          >
            {initialEvent ? 'Edit Event' : 'New Event'}
          </h2>
          <button
            type="button"
            onClick={handleSubmit}
            className="flex items-center gap-2 font-semibold disabled:cursor-not-allowed"
            style={{
              color:
                !title || !date || isSubmitting
                  ? 'var(--ctp-overlay0)'
                  : 'var(--ctp-sapphire)',
            }}
            disabled={!title || !date || isSubmitting}
          >
            {isSubmitting && <Loader2 className="h-4 w-4 animate-spin" />}
            {initialEvent ? 'Save' : 'Add'}
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-4 p-5">
          {/* Title */}
          <div>
            <label htmlFor="title" className="sr-only">
              Event Title
            </label>
            <input
              type="text"
              id="title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Event title"
              className="w-full rounded-lg px-4 py-3 text-base focus:ring-2 focus:outline-none"
              style={{
                backgroundColor: 'var(--ctp-mantle)',
                border: '1px solid var(--ctp-surface0)',
                color: 'var(--ctp-text)',
              }}
              required
              autoFocus
            />
          </div>

          {/* Date */}
          <div>
            <label
              htmlFor="date-trigger"
              className="mb-2 block text-sm font-medium"
              style={{ color: 'var(--ctp-subtext0)' }}
            >
              Date
            </label>
            <button
              id="date-trigger"
              type="button"
              onClick={() => setShowDatePicker(!showDatePicker)}
              aria-haspopup="dialog"
              aria-expanded={showDatePicker}
              aria-controls="date-picker-content"
              className="w-full rounded-lg px-4 py-3 text-left text-base focus:ring-2 focus:outline-none"
              style={{
                backgroundColor: 'var(--ctp-mantle)',
                border: '1px solid var(--ctp-surface0)',
                color: 'var(--ctp-text)',
              }}
            >
              {new Date(date + 'T00:00:00').toLocaleDateString('en-US', {
                weekday: 'long',
                year: 'numeric',
                month: 'long',
                day: 'numeric',
              })}
            </button>

            {showDatePicker && (
              <div
                id="date-picker-content"
                role="dialog"
                aria-label="Choose a date"
                className="mt-2 rounded-lg p-4"
                style={{
                  backgroundColor: 'var(--ctp-mantle)',
                  border: '1px solid var(--ctp-surface0)',
                }}
              >
                <div
                  className="mb-3 text-center font-semibold"
                  style={{ color: 'var(--ctp-text)' }}
                >
                  {monthNames[month]} {year}
                </div>
                <div className="grid grid-cols-7 gap-1 text-center text-sm">
                  {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map(
                    (day) => (
                      <div
                        key={day}
                        className="py-2 font-medium"
                        style={{ color: 'var(--ctp-subtext0)' }}
                      >
                        {day}
                      </div>
                    )
                  )}
                  {days.map((day, index) => {
                    // Fix timezone issue by manually constructing YYYY-MM-DD
                    const currentDayStr = day
                      ? day.toString().padStart(2, '0')
                      : ''
                    const currentMonthStr = (month + 1)
                      .toString()
                      .padStart(2, '0')
                    const dateStr = `${year}-${currentMonthStr}-${currentDayStr}`
                    const isSelected = day ? dateStr === date : false

                    const label = day
                      ? `${monthNames[month]} ${day}, ${year}${isSelected ? ', selected' : ''}`
                      : undefined

                    return (
                      <div key={index}>
                        {day ? (
                          <button
                            type="button"
                            onClick={() => handleDateSelect(day)}
                            aria-label={label}
                            className="flex aspect-square w-full items-center justify-center rounded-lg"
                            style={{
                              backgroundColor: isSelected
                                ? 'var(--ctp-sapphire)'
                                : 'transparent',
                              color: isSelected
                                ? 'var(--ctp-crust)'
                                : 'var(--ctp-text)',
                              fontWeight: isSelected ? 600 : 400,
                            }}
                          >
                            {day}
                          </button>
                        ) : (
                          <div />
                        )}
                      </div>
                    )
                  })}
                </div>
              </div>
            )}
          </div>

          {/* Time Picker */}
          <div>
            <label
              className="mb-2 block text-sm font-medium"
              style={{ color: 'var(--ctp-subtext0)' }}
            >
              Time
            </label>
            <div
              className="relative h-48 overflow-hidden rounded-lg"
              style={{
                backgroundColor: 'var(--ctp-mantle)',
                border: '1px solid var(--ctp-surface0)',
              }}
            >
              <div
                className="pointer-events-none absolute inset-x-0 top-1/2 z-10 h-12 -translate-y-1/2"
                style={{
                  backgroundColor: 'rgba(32, 159, 181, 0.1)',
                  borderTop: '2px solid var(--ctp-sapphire)',
                  borderBottom: '2px solid var(--ctp-sapphire)',
                }}
              />
              <div
                ref={timePickerRef}
                onScroll={() => {
                  isScrollingTimeRef.current = true
                }}
                onScrollEnd={handleTimeScrollEnd}
                onTouchEnd={handleTimeScrollEnd}
                onMouseUp={handleTimeScrollEnd}
                className="scrollbar-hide h-full overflow-y-scroll"
              >
                <div className="h-24" />
                {timeOptions.map((timeOption, index) => (
                  <div
                    key={index}
                    className="flex h-12 items-center justify-center text-xl select-none"
                    style={{
                      color: 'var(--ctp-text)',
                      opacity: timeOption === time ? 1 : 0.4,
                      transition: 'opacity 0.2s',
                    }}
                  >
                    {timeOption}
                  </div>
                ))}
                <div className="h-24" />
              </div>
            </div>
          </div>

          {/* Duration Picker */}
          <div>
            <label
              className="mb-2 block text-sm font-medium"
              style={{ color: 'var(--ctp-subtext0)' }}
            >
              Duration
            </label>
            <div
              className="relative h-48 overflow-hidden rounded-lg"
              style={{
                backgroundColor: 'var(--ctp-mantle)',
                border: '1px solid var(--ctp-surface0)',
              }}
            >
              <div
                className="pointer-events-none absolute inset-x-0 top-1/2 z-10 h-12 -translate-y-1/2"
                style={{
                  backgroundColor: 'rgba(32, 159, 181, 0.1)',
                  borderTop: '2px solid var(--ctp-sapphire)',
                  borderBottom: '2px solid var(--ctp-sapphire)',
                }}
              />
              <div
                ref={durationPickerRef}
                onScroll={() => {
                  isScrollingDurationRef.current = true
                }}
                onScrollEnd={handleDurationScrollEnd}
                onTouchEnd={handleDurationScrollEnd}
                onMouseUp={handleDurationScrollEnd}
                className="scrollbar-hide h-full overflow-y-scroll"
              >
                <div className="h-24" />
                {durationOptions.map((durationOption, index) => (
                  <div
                    key={index}
                    className="flex h-12 items-center justify-center text-xl select-none"
                    style={{
                      color: 'var(--ctp-text)',
                      opacity: durationOption.value === duration ? 1 : 0.4,
                      transition: 'opacity 0.2s',
                    }}
                  >
                    {durationOption.label}
                  </div>
                ))}
                <div className="h-24" />
              </div>
            </div>
          </div>

          {/* Location */}
          <div>
            <label
              htmlFor="location"
              className="mb-2 block text-sm font-medium"
              style={{ color: 'var(--ctp-subtext0)' }}
            >
              Location (optional)
            </label>
            <div className="relative">
              <MapPin
                className="absolute top-1/2 left-3 h-5 w-5 -translate-y-1/2"
                style={{ color: 'var(--ctp-overlay1)' }}
              />
              <input
                id="location"
                type="text"
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                placeholder="Add location"
                className="w-full rounded-lg py-3 pr-4 pl-11 text-base focus:ring-2 focus:outline-none"
                style={{
                  backgroundColor: 'var(--ctp-mantle)',
                  border: '1px solid var(--ctp-surface0)',
                  color: 'var(--ctp-text)',
                }}
              />
            </div>
          </div>
        </form>
      </div>
    </div>
  )
}
