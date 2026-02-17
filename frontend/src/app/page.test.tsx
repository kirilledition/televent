import { render, screen, fireEvent } from '@testing-library/react'
import CalendarPage from './page'
import { describe, it, expect, vi } from 'vitest'

// Mock useRouter
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

describe('CalendarPage', () => {
  it('renders correctly with dummy events', () => {
    render(<CalendarPage />)
    expect(screen.getByText('Calendar')).toBeInTheDocument()
    expect(screen.getByText('New event')).toBeInTheDocument()
    // Check if dummy events are rendered
    expect(screen.getByText('Team Meeting')).toBeInTheDocument()
  })

  it('navigates to create page on button click', () => {
    render(<CalendarPage />)
    fireEvent.click(screen.getByText('New event'))
    expect(mockPush).toHaveBeenCalledWith('/create')
  })

  it('deletes an event locally', () => {
    render(<CalendarPage />)

    // Mock window.confirm
    vi.spyOn(window, 'confirm').mockImplementation(() => true)

    const deleteBtns = screen.getAllByRole('button', { name: /Delete event/i })
    const initialCount = deleteBtns.length

    fireEvent.click(deleteBtns[0])

    // Should be one less delete button
    const newDeleteBtns = screen.getAllByRole('button', {
      name: /Delete event/i,
    })
    expect(newDeleteBtns.length).toBe(initialCount - 1)
  })

  it('navigates to edit page when event is clicked', () => {
    render(<CalendarPage />)
    const eventItems = screen.getAllByRole('button', { name: /Edit event/i })
    fireEvent.click(eventItems[0])
    // The dummy event ID is '1'
    expect(mockPush).toHaveBeenCalledWith('/event/1')
  })
})
