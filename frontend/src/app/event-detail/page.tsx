'use client'

import { useQuery } from '@tanstack/react-query'
import { api } from '@/lib/api'
import { mapApiEventToUiEvent } from '@/lib/mappers'
import EventDetailPageClient from './EventDetailPageClient'
import { useSearchParams, useRouter } from 'next/navigation'
import { Loader2 } from 'lucide-react'
import { Suspense } from 'react'

function EventDetailContent() {
  const searchParams = useSearchParams()
  const id = searchParams.get('id')
  const router = useRouter()

  const { data: eventData, isLoading, error } = useQuery({
    queryKey: ['events', id],
    queryFn: () => (id ? api.getEvent(id) : Promise.reject('No ID')),
    enabled: !!id,
  })

  if (!id) {
    return (
      <div className="flex h-screen items-center justify-center text-gray-500">
        Invalid Event ID
      </div>
    )
  }

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

  if (error || !eventData) {
    return (
      <div
        className="flex h-screen flex-col items-center justify-center gap-4 text-center"
        style={{ backgroundColor: 'var(--ctp-base)', color: 'var(--ctp-red)' }}
      >
        <p>Event not found or error loading event.</p>
        <button
          onClick={() => router.back()}
          className="text-sapphire underline"
        >
          Go Back
        </button>
      </div>
    )
  }

  const event = mapApiEventToUiEvent(eventData)

  return <EventDetailPageClient event={event} />
}

export default function EventDetailPage() {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <EventDetailContent />
    </Suspense>
  )
}
