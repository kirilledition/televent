'use client'

import { EventDetail } from '../../components/EventDetail'
import { useRouter } from 'next/navigation'
import { Event } from '@/types/event'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@/lib/api'

export default function EventDetailPageClient({ event }: { event: Event }) {
  const router = useRouter()
  const queryClient = useQueryClient()

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteEvent(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['events'] })
      router.back()
    },
  })

  const handleDelete = (id: string) => {
    deleteMutation.mutate(id)
  }

  const handleEdit = (event: Event) => {
    router.push(`/edit-event?id=${event.id}`)
  }

  return (
    <EventDetail event={event} onDelete={handleDelete} onEdit={handleEdit} />
  )
}
