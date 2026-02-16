import { render, screen, fireEvent } from '@testing-library/react'
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

describe('EventItem', () => {
  const onDelete = vi.fn()
  const onEdit = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    vi.spyOn(window, 'confirm').mockImplementation(() => true)
  })

  it('renders event details', () => {
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    expect(screen.getByText('Test Event')).toBeInTheDocument()
    expect(screen.getByText('10:00')).toBeInTheDocument()
    expect(screen.getByText(/1h 30m/)).toBeInTheDocument() // 90 min = 1h 30m
    expect(screen.getByText('Office')).toBeInTheDocument()
  })

  it('calls onEdit when clicked', () => {
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    fireEvent.click(screen.getByText('Test Event'))
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('calls onEdit when Enter key is pressed', () => {
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    const element = screen.getByRole('button', { name: /Edit event/i })
    fireEvent.keyDown(element, { key: 'Enter' })
    expect(onEdit).toHaveBeenCalledWith(mockEvent)
  })

  it('calls onDelete when delete button is clicked and confirmed', () => {
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    const deleteBtn = screen.getByRole('button', { name: /Delete event/i })
    fireEvent.click(deleteBtn)
    expect(window.confirm).toHaveBeenCalled()
    expect(onDelete).toHaveBeenCalledWith(mockEvent.id)
  })

  it('does not call onDelete when delete is cancelled', () => {
    vi.spyOn(window, 'confirm').mockImplementation(() => false)
    render(<EventItem event={mockEvent} onDelete={onDelete} onEdit={onEdit} />)
    const deleteBtn = screen.getByRole('button', { name: /Delete event/i })
    fireEvent.click(deleteBtn)
    expect(window.confirm).toHaveBeenCalled()
    expect(onDelete).not.toHaveBeenCalled()
  })
})
