'use client'

import { useQuery } from '@tanstack/react-query'
import { EventForm } from '@/components/EventForm'
import { api } from '@/lib/api'
import { useRouter, useSearchParams } from 'next/navigation'
import { Suspense } from 'react'

function EditEventContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const id = searchParams.get('id')

  const {
    data: event,
    isLoading,
    error,
  } = useQuery({
    queryKey: ['events', id],
    queryFn: () => (id ? api.getEvent(id) : Promise.reject('No ID')),
    enabled: !!id,
  })

  // If no ID is provided, go back
  if (!id) {
    return (
      <div className="p-8 text-center text-red">
        Invalid Event ID
        <br />
        <button
          onClick={() => router.back()}
          className="text-sapphire mt-4 underline"
        >
          Go Back
        </button>
      </div>
    )
  }

  if (isLoading)
    return <div className="text-subtext0 p-8 text-center">Loading...</div>
  if (error)
    return (
      <div className="p-8 text-center">
        <p className="text-red mb-4">Error loading event</p>
        <button onClick={() => router.push('/')} className="btn-secondary">
          Go Back
        </button>
      </div>
    )

  return (
    <>
      <header className="mb-6 flex items-center gap-3">
        <button
          onClick={() => router.back()}
          className="text-sapphire hover:text-sky -ml-1 p-1 transition-colors"
          aria-label="Go back"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m15 18-6-6 6-6" />
          </svg>
        </button>
        <h1 className="text-sapphire text-xl font-bold">Edit Event</h1>
      </header>

      {event && <EventForm initialData={event} isEditing />}
    </>
  )
}

export default function EditEventPage() {
  return (
    <main className="bg-base text-text min-h-screen p-4 pb-20">
      <Suspense fallback={<div>Loading...</div>}>
        <EditEventContent />
      </Suspense>
    </main>
  )
}
