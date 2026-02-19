import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { EventItem } from './EventItem'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { Event } from '@/types/event'

const mockEvent: Event = {
  id: '1',
  title: 'Test Event',
  date: '2023-10-01',
  time: '10:00',
  duration: 90,
  location: 'Office',
}

// Mock ResizeObserver for Radix UI
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

describe('EventItem', () => {
  const onDelete = vi.fn()
  const onEdit = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    // window.confirm is no longer used, but if mocked, it shouldn't be called.
  })

  it('renders event details', () => {
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    expect(screen.getByText('Test Event')).toBeInTheDocument()
    expect(screen.getByText('10:00')).toBeInTheDocument()
    expect(screen.getByText(/1h 30m/)).toBeInTheDocument() // 90 min = 1h 30m
    expect(screen.getByText('Office')).toBeInTheDocument()
  })

  it('calls onEdit when clicked', async () => {
    const user = userEvent.setup()
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    await user.click(screen.getByText('Test Event'))
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('calls onEdit when Enter key is pressed', async () => {
    const user = userEvent.setup()
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    const element = screen.getByRole('button', { name: /Edit event/i })
    element.focus()
    await user.keyboard('{Enter}')
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('calls onDelete when delete button is clicked and confirmed via dialog', async () => {
    const user = userEvent.setup()
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)

    const deleteBtn = screen.getByRole('button', { name: /Delete event/i })
    await user.click(deleteBtn)

    // Dialog should open
    const confirmBtn = await screen.findByRole('button', { name: 'Delete' })
    await user.click(confirmBtn)

    expect(onDelete).toHaveBeenCalledWith(mockEvent.id)
  })

  it('does not call onDelete when delete is cancelled via dialog', async () => {
    const user = userEvent.setup()
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)

    const deleteBtn = screen.getByRole('button', { name: /Delete event/i })
    await user.click(deleteBtn)

    // Dialog should open
    const cancelBtn = await screen.findByRole('button', { name: 'Cancel' })
    await user.click(cancelBtn)

    expect(onDelete).not.toHaveBeenCalled()
  })
})
