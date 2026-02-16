import { render, screen, fireEvent } from '@testing-library/react'
import { EventDetail } from './EventDetail'
import { describe, it, expect, vi } from 'vitest'
import { Event } from '@/types/event'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
  }),
}))

const mockEvent: Event = {
  id: '1',
  title: 'Test Event',
  date: '2023-10-01',
  time: '10:00',
  duration: 90,
  location: 'Office',
  description: 'Detailed description',
}

describe('EventDetail', () => {
  it('renders event details', () => {
    const onDelete = vi.fn()
    const onEdit = vi.fn()
    render(
      <EventDetail event={mockEvent} onDelete={onDelete} onEdit={onEdit} />
    )

    expect(screen.getByText('Test Event')).toBeInTheDocument()
    expect(screen.getByText('Office')).toBeInTheDocument()
    expect(screen.getByText('Detailed description')).toBeInTheDocument()
    // Check duration formatting
    expect(screen.getByText('1h 30m duration')).toBeInTheDocument()
  })

  it('calls onDelete', () => {
    const onDelete = vi.fn()
    render(
      <EventDetail event={mockEvent} onDelete={onDelete} onEdit={vi.fn()} />
    )
    fireEvent.click(screen.getByText('Delete Event'))
    expect(onDelete).toHaveBeenCalledWith(mockEvent.id)
  })

  it('calls onEdit', () => {
    const onEdit = vi.fn()
    render(<EventDetail event={mockEvent} onDelete={vi.fn()} onEdit={onEdit} />)
    fireEvent.click(screen.getByText('Edit'))
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('navigates back', () => {
    render(
      <EventDetail event={mockEvent} onDelete={vi.fn()} onEdit={vi.fn()} />
    )
    // Find back button (it has no text, so maybe by role or test id, but it renders an icon)
    // The button is the first button in the header.
    // Let's use getByRole('button') logic or finding the one calling router.back()
    // It has className "-ml-2 rounded-full..."
    const buttons = screen.getAllByRole('button')
    fireEvent.click(buttons[0]) // First button is usually back in the header
    expect(mockBack).toHaveBeenCalled()
  })
})
