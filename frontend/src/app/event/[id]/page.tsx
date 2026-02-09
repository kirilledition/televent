import { DUMMY_EVENTS } from '@/lib/dummy-data'
import EventDetailPageClient from './EventDetailPageClient'

export async function generateStaticParams() {
  return DUMMY_EVENTS.map((event) => ({
    id: event.id,
  }))
}

export default async function EventDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const resolvedParams = await params
  const id = resolvedParams.id

  const event = DUMMY_EVENTS.find(e => e.id === id)

  if (!event) {
    return (
      <div className="flex h-screen items-center justify-center text-gray-500">
        Event not found
      </div>
    )
  }

  return <EventDetailPageClient event={event} />
}
