import { render, screen, fireEvent } from '@testing-library/react'
import EventDetailPageClient from './EventDetailPageClient'
import { describe, it, expect, vi } from 'vitest'
import { Event } from '@/types/event'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
    push: vi.fn(),
  }),
}))

const mockEvent: Event = {
  id: '1',
  title: 'Test Event',
  date: '2023-10-01',
  time: '10:00',
  duration: 90,
  location: 'Office',
}

describe('EventDetailPageClient', () => {
  it('renders correctly', () => {
    render(<EventDetailPageClient event={mockEvent} />)
    expect(screen.getByText('Test Event')).toBeInTheDocument()
  })

  it('handles delete', () => {
    // Spy on console.log if needed, but mainly check navigation
    render(<EventDetailPageClient event={mockEvent} />)
    fireEvent.click(screen.getByText('Delete Event'))
    expect(mockBack).toHaveBeenCalled()
  })
})
