'use client'

import { CreateEvent } from '../../components/CreateEvent'
import { useRouter } from 'next/navigation'
import { Event } from '@/types/event'

export default function CreateEventPage() {
  const router = useRouter()

  const handleCreate = (event: Omit<Event, 'id'>) => {
    console.log('Creating event:', event)
    router.back()
  }

  return (
    <CreateEvent
      onClose={() => router.back()}
      onCreate={handleCreate}
    />
  )
}
