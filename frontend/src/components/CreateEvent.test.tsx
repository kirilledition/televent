import { render, screen, fireEvent, within } from '@testing-library/react'
import { CreateEvent } from './CreateEvent'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Mock DOM APIs
Element.prototype.scrollIntoView = vi.fn()
Element.prototype.scrollTo = vi.fn()

describe('CreateEvent', () => {
  const onClose = vi.fn()
  const onCreate = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2023-10-15T10:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders correctly', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)
    expect(screen.getByText('New Event')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Event title')).toBeInTheDocument()
    // Check for date text loosely
    expect(
      screen.getByText((content) => content.includes('October 15, 2023'))
    ).toBeInTheDocument()
  })

  it('calls onClose', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)
    fireEvent.click(screen.getByText('Cancel'))
    expect(onClose).toHaveBeenCalled()
  })

  it('creates event with default values', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)

    const titleInput = screen.getByPlaceholderText('Event title')
    fireEvent.change(titleInput, { target: { value: 'My Event' } })

    fireEvent.click(screen.getByText('Add'))

    expect(onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'My Event',
        duration: 45,
      })
    )
  })

  it('updates duration', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)

    const listbox = screen.getByRole('listbox', { name: 'Select duration' })
    const option90 = within(listbox).getByText('1h 30m')
    fireEvent.click(option90)

    const titleInput = screen.getByPlaceholderText('Event title')
    fireEvent.change(titleInput, { target: { value: 'Long Event' } })
    fireEvent.click(screen.getByText('Add'))

    expect(onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        duration: 90,
      })
    )
  })

  it('updates time', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)

    const listbox = screen.getByRole('listbox', { name: 'Select time' })
    const timeOption = within(listbox).getByText('12:00')
    fireEvent.click(timeOption)

    const titleInput = screen.getByPlaceholderText('Event title')
    fireEvent.change(titleInput, { target: { value: 'Lunch' } })
    fireEvent.click(screen.getByText('Add'))

    expect(onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        time: '12:00',
      })
    )
  })

  it('opens calendar and selects date', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)

    // Find button containing date
    const dateBtn = screen.getByText((content) =>
      content.includes('October 15, 2023')
    )
    fireEvent.click(dateBtn)

    // Calendar dialog
    const dialog = screen.getByRole('dialog', { name: 'Calendar' })

    // Select 20th. Be careful not to select "20" from time/duration.
    // The calendar dates are usually buttons inside the dialog.
    const day20 = within(dialog).getByRole('button', { name: '20' })
    fireEvent.click(day20)

    // Verify date text updated
    expect(
      screen.getByText((content) => content.includes('October 20, 2023'))
    ).toBeInTheDocument()

    // Submit
    const titleInput = screen.getByPlaceholderText('Event title')
    fireEvent.change(titleInput, { target: { value: 'Future Event' } })
    fireEvent.click(screen.getByText('Add'))

    // Note: The component sends YYYY-MM-DD string.
    // If we are in UTC, 2023-10-20.
    // But verify the property name. Component sets Mon Feb 16 16:58:53 UTC 2026 state.
    expect(onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        date: expect.stringContaining('2023-10-20'),
      })
    )
  })

  it('updates location', () => {
    render(<CreateEvent onClose={onClose} onCreate={onCreate} />)

    const locInput = screen.getByPlaceholderText('Add location')
    fireEvent.change(locInput, { target: { value: 'Remote' } })

    const titleInput = screen.getByPlaceholderText('Event title')
    fireEvent.change(titleInput, { target: { value: 'Meeting' } })
    fireEvent.click(screen.getByText('Add'))

    expect(onCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        location: 'Remote',
      })
    )
  })
})
