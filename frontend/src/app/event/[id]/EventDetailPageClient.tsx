'use client'

import { EventDetail } from '../../../components/EventDetail'
import { useRouter } from 'next/navigation'
import { Event } from '@/types/event'

export default function EventDetailPageClient({ event }: { event: Event }) {
  const router = useRouter()

  const handleDelete = (id: string) => {
    console.log('Deleting event:', id)
    router.back()
  }

  const handleEdit = (event: Event) => {
    console.log('Editing event:', event)
  }

  return (
    <EventDetail
      event={event}
      onDelete={handleDelete}
      onEdit={handleEdit}
    />
  )
}
