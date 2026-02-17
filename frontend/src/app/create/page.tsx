'use client'

import { EventForm } from '@/components/EventForm'
import { useRouter } from 'next/navigation'

export default function CreateEventPage() {
  const router = useRouter()

  return (
    <div
      className="mx-auto min-h-screen w-full sm:max-w-md"
      style={{ backgroundColor: 'var(--ctp-base)' }}
    >
      {/* Header */}
      <div
        className="sticky top-0 z-10 flex items-center justify-between px-5 py-4"
        style={{
          backgroundColor: 'var(--ctp-base)',
          borderBottom: '1px solid var(--ctp-surface0)',
        }}
      >
        <button
          type="button"
          onClick={() => router.back()}
          className="font-medium"
          style={{ color: 'var(--ctp-sapphire)' }}
        >
          Cancel
        </button>
        <h1
          className="text-lg font-semibold"
          style={{ color: 'var(--ctp-text)' }}
        >
          New Event
        </h1>
        {/* Placeholder for symmetry */}
        <div className="w-12" />
      </div>

      <EventForm />
    </div>
  )
}
