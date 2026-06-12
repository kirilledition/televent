import { render, screen, fireEvent } from '@testing-library/react'
import { EventList } from './EventList'
import { describe, it, expect, vi } from 'vitest'
import { Event } from '@/types/event'

function localDateString(offsetDays: number) {
  const date = new Date()
  date.setDate(date.getDate() + offsetDays)
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

const mockEvents: Event[] = [
  {
    id: '1',
    title: 'Event 1',
    date: localDateString(0),
    time: '10:00',
    description: 'Description 1',
  },
  {
    id: '2',
    title: 'Event 2',
    date: localDateString(1),
    time: '14:00',
  },
]

describe('EventList', () => {
  it('renders events grouped by date', () => {
    const onDelete = vi.fn()
    const onEdit = vi.fn()
    render(
      <EventList
        events={mockEvents}
        onDeleteEvent={onDelete}
        onEditEvent={onEdit}
      />
    )

    expect(screen.getByText('Today')).toBeInTheDocument()
    expect(screen.getByText('Tomorrow')).toBeInTheDocument()
    expect(screen.getByText('Event 1')).toBeInTheDocument()
    expect(screen.getByText('Event 2')).toBeInTheDocument()
  })

  it('renders empty state when no events', () => {
    render(
      <EventList events={[]} onDeleteEvent={vi.fn()} onEditEvent={vi.fn()} />
    )
    expect(screen.getByText(/No events yet/i)).toBeInTheDocument()
  })

  it('calls onEditEvent when event is clicked', () => {
    const onDelete = vi.fn()
    const onEdit = vi.fn()
    render(
      <EventList
        events={mockEvents}
        onDeleteEvent={onDelete}
        onEditEvent={onEdit}
      />
    )

    fireEvent.click(screen.getByText('Event 1'))
    expect(onEdit).toHaveBeenCalledWith(mockEvents[0])
  })
})
