'use client'

import { useState, useMemo } from 'react'
import { useRouter } from 'next/navigation'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  api,
  CreateEventRequest,
  UpdateEventRequest,
  EventResponse,
} from '@/lib/api'
import {
  addMinutes,
  format,
  differenceInMinutes,
  roundToNearestMinutes,
} from 'date-fns'

interface EventFormProps {
  initialData?: EventResponse
  isEditing?: boolean
}

export function EventForm({ initialData, isEditing = false }: EventFormProps) {
  const router = useRouter()
  const queryClient = useQueryClient()
  const [error, setError] = useState<string | null>(null)

  // Initial values logic
  const initialStart = useMemo(() => {
    if (initialData?.start) return new Date(initialData.start)
    // Round up to next 5 minutes
    return roundToNearestMinutes(new Date(), {
      nearestTo: 5,
      roundingMethod: 'ceil',
    })
  }, [initialData])

  const initialDuration = useMemo(() => {
    if (initialData?.start && initialData?.end) {
      return differenceInMinutes(
        new Date(initialData.end),
        new Date(initialData.start)
      )
    }
    return 45 // Default duration
  }, [initialData])

  const [formData, setFormData] = useState({
    summary: initialData?.summary || '',
    description: initialData?.description || '',
    location: initialData?.location || '',
    start: format(initialStart, "yyyy-MM-dd'T'HH:mm"),
    duration: initialDuration,
    is_all_day: initialData?.is_all_day || false,
    timezone: (initialData?.timezone as string) || 'UTC',
  })

  // Generate duration options (5 min steps)
  const durationOptions = useMemo(() => {
    const options = []
    // Up to 12 hours (720 mins)
    for (let i = 5; i <= 720; i += 5) {
      options.push(i)
    }
    return options
  }, [])

  const createMutation = useMutation({
    mutationFn: (data: CreateEventRequest) => api.createEvent(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] })
      router.push('/')
      router.refresh()
    },
    onError: (err: Error) => setError(err.message),
  })

  const updateMutation = useMutation({
    mutationFn: (data: UpdateEventRequest) =>
      api.updateEvent(initialData!.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] })
      queryClient.invalidateQueries({ queryKey: ['events', initialData!.id] })
      router.push('/')
      router.refresh()
    },
    onError: (err: Error) => setError(err.message),
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!formData.summary) {
      setError('Summary is required')
      return
    }

    const startDate = new Date(formData.start)
    const endDate = addMinutes(startDate, formData.duration)

    const payload = {
      summary: formData.summary,
      description: formData.description,
      location: formData.location,
      start: startDate.toISOString(),
      end: endDate.toISOString(),
      is_all_day: formData.is_all_day,
      timezone: formData.timezone,
    }

    if (isEditing && initialData) {
      updateMutation.mutate(payload)
    } else {
      createMutation.mutate({
        ...payload,
        uid: crypto.randomUUID(), // Generate UID on frontend for new events
      })
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-5">
      {error && (
        <div className="bg-red/20 text-red border-red rounded-lg border p-3 text-sm">
          {error}
        </div>
      )}

      <div>
        <label
          htmlFor="summary"
          className="text-muted mb-1.5 block text-sm font-medium"
        >
          Title <span className="text-red">*</span>
        </label>
        <input
          id="summary"
          type="text"
          value={formData.summary}
          onChange={(e) =>
            setFormData({ ...formData, summary: e.target.value })
          }
          placeholder="Event title"
          required
          className="bg-surface text-text border-border focus:border-primary w-full rounded-lg p-3 transition-colors outline-none"
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <label
            htmlFor="start"
            className="text-muted mb-1.5 block text-sm font-medium"
          >
            Start <span className="text-red">*</span>
          </label>
          <input
            id="start"
            type="datetime-local"
            value={formData.start}
            onChange={(e) =>
              setFormData({ ...formData, start: e.target.value })
            }
            required
            className="bg-surface text-text border-border focus:border-primary w-full rounded-lg p-3 transition-colors outline-none"
          />
        </div>

        <div>
          <label
            htmlFor="duration"
            className="text-muted mb-1.5 block text-sm font-medium"
          >
            Duration (min)
          </label>
          <select
            id="duration"
            value={formData.duration}
            onChange={(e) =>
              setFormData({ ...formData, duration: Number(e.target.value) })
            }
            className="bg-surface text-text border-border focus:border-primary w-full appearance-none rounded-lg p-3 transition-colors outline-none"
          >
            {durationOptions.map((mins) => (
              <option key={mins} value={mins}>
                {mins} min{' '}
                {mins >= 60 &&
                  `(${Math.floor(mins / 60)}h ${mins % 60 > 0 ? (mins % 60) + 'm' : ''})`}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div>
        <label
          htmlFor="description"
          className="text-muted mb-1.5 block text-sm font-medium"
        >
          Description
        </label>
        <textarea
          id="description"
          value={formData.description}
          onChange={(e) =>
            setFormData({ ...formData, description: e.target.value })
          }
          placeholder="Details..."
          rows={3}
          className="bg-surface text-text border-border focus:border-primary w-full rounded-lg p-3 transition-colors outline-none"
        />
      </div>

      <div>
        <label
          htmlFor="location"
          className="text-muted mb-1.5 block text-sm font-medium"
        >
          Location
        </label>
        <input
          id="location"
          type="text"
          value={formData.location}
          onChange={(e) =>
            setFormData({ ...formData, location: e.target.value })
          }
          placeholder="Where?"
          className="bg-surface text-text border-border focus:border-primary w-full rounded-lg p-3 transition-colors outline-none"
        />
      </div>

      <div className="flex items-center gap-3 py-2">
        <input
          type="checkbox"
          id="all_day"
          checked={formData.is_all_day}
          onChange={(e) =>
            setFormData({ ...formData, is_all_day: e.target.checked })
          }
          className="border-border text-primary focus:ring-primary bg-surface h-5 w-5 rounded"
        />
        <label
          htmlFor="all_day"
          className="text-text text-sm font-medium select-none"
        >
          All Day Event
        </label>
      </div>

      <div className="mt-4 flex gap-3">
        <button
          type="button"
          onClick={() => router.back()}
          className="btn-secondary flex-1"
        >
          Cancel
        </button>
        <button
          type="submit"
          className="btn-primary flex-1"
          disabled={createMutation.isPending || updateMutation.isPending}
        >
          {createMutation.isPending || updateMutation.isPending
            ? 'Saving...'
            : isEditing
              ? 'Update Event'
              : 'Create Event'}
        </button>
      </div>
    </form>
  )
}
