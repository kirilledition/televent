'use client'

import { EventForm } from '@/components/EventForm'
import { useRouter } from 'next/navigation'

export default function CreateEventPage() {
  const router = useRouter()

  return (
    <main className="bg-base text-text min-h-screen p-4 pb-20">
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
        <h1 className="text-sapphire text-xl font-bold">New Event</h1>
      </header>

      <EventForm />
    </main>
  )
}
