import { render, screen, fireEvent } from '@testing-library/react'
import { EventList } from './EventList'
import { describe, it, expect, vi } from 'vitest'
import { Event } from '@/types/event'

const mockEvents: Event[] = [
  {
    id: '1',
    title: 'Event 1',
    date: new Date().toISOString().split('T')[0], // Today
    time: '10:00',
    description: 'Description 1',
  },
  {
    id: '2',
    title: 'Event 2',
    date: new Date(Date.now() + 86400000).toISOString().split('T')[0], // Tomorrow
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
    expect(
      screen.getByText(/Create your first event to get started/i)
    ).toBeInTheDocument()
  })

  it('renders create button in empty state when onCreateEvent is provided', () => {
    const onCreate = vi.fn()
    render(
      <EventList
        events={[]}
        onDeleteEvent={vi.fn()}
        onEditEvent={vi.fn()}
        onCreateEvent={onCreate}
      />
    )
    const createBtn = screen.getByRole('button', { name: /Create Event/i })
    expect(createBtn).toBeInTheDocument()
    fireEvent.click(createBtn)
    expect(onCreate).toHaveBeenCalled()
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
