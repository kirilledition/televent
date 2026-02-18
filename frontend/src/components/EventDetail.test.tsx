import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EventDetail } from './EventDetail'
import { describe, it, expect, vi } from 'vitest'
import { Event } from '@/types/event'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
  }),
}))

// Mock ResizeObserver for Radix UI
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

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

  it('calls onDelete via confirmation dialog', async () => {
    const user = userEvent.setup()
    const onDelete = vi.fn()
    render(
      <EventDetail event={mockEvent} onDelete={onDelete} onEdit={vi.fn()} />
    )

    // Click the initial delete button
    await user.click(screen.getByText('Delete Event'))

    // Check that dialog opens
    const dialogTitle = await screen.findByRole('heading', {
      name: 'Delete Event',
    })
    expect(dialogTitle).toBeInTheDocument()

    // Click the confirmation button inside the dialog
    // We target the "Delete" button specifically. Since there might be multiple "Delete" texts
    // (e.g. if the main button is still visible to screen reader but aria-hidden),
    // we should be careful. But "Delete Event" != "Delete".
    // "Delete" is the text of the action button.
    const deleteButton = screen.getByRole('button', { name: 'Delete' })
    await user.click(deleteButton)

    expect(onDelete).toHaveBeenCalledWith(mockEvent.id)
  })

  it('calls onEdit', async () => {
    const user = userEvent.setup()
    const onEdit = vi.fn()
    render(<EventDetail event={mockEvent} onDelete={vi.fn()} onEdit={onEdit} />)
    await user.click(screen.getByText('Edit'))
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('navigates back', async () => {
    const user = userEvent.setup()
    render(
      <EventDetail event={mockEvent} onDelete={vi.fn()} onEdit={vi.fn()} />
    )
    // Find back button by aria-label
    const backButton = screen.getByLabelText('Go back')
    await user.click(backButton)
    expect(mockBack).toHaveBeenCalled()
  })
})
