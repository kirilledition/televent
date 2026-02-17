import { render, screen, fireEvent } from '@testing-library/react'
import CreateEventPage from './page'
import { describe, it, expect, vi } from 'vitest'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
  }),
}))

describe('CreateEventPage', () => {
  it('renders correctly', () => {
    render(<CreateEventPage />)
    expect(screen.getByText('New Event')).toBeInTheDocument()
  })

  it('navigates back on close', () => {
    render(<CreateEventPage />)
    fireEvent.click(screen.getByText('Cancel'))
    expect(mockBack).toHaveBeenCalled()
  })
})
