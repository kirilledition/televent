import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import CalendarPage from './page'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { api } from '@/lib/api'

// Mock useRouter
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

// Mock API
vi.mock('@/lib/api', async (importOriginal) => {
  const actual = await importOriginal()
  return {
    ...actual,
    api: {
      getEvents: vi.fn(),
      deleteEvent: vi.fn(),
    },
  }
})

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

const mockEvents = [
  {
    id: '1',
    summary: 'Team Meeting',
    start: '2023-10-27T10:00:00Z',
    end: '2023-10-27T11:00:00Z',
    location: 'Office',
  },
]

describe('CalendarPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ;(api.getEvents as any).mockResolvedValue(mockEvents)
  })

  it('renders correctly with dummy events', async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )

    // Wait for loading to finish first
    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument()
    })

    expect(screen.getByText('New event')).toBeInTheDocument()

    // Check for event
    expect(screen.getByText('Team Meeting')).toBeInTheDocument()
  })

  it('navigates to create page on button click', async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )

    // Wait for loading to finish
    await waitFor(() => {
      expect(screen.getByText('New event')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('New event'))
    expect(mockPush).toHaveBeenCalledWith('/create')
  })

  it('deletes an event locally', async () => {
    ;(api.deleteEvent as any).mockResolvedValue({})

    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Team Meeting')).toBeInTheDocument()
    })

    const deleteBtn = screen.getByRole('button', { name: /Delete event/i })
    fireEvent.click(deleteBtn)

    // Confirm deletion in dialog
    const confirmBtn = screen.getByText('Delete')
    fireEvent.click(confirmBtn)

    await waitFor(() => {
      expect(api.deleteEvent).toHaveBeenCalledWith('1')
    })
  })

  it('navigates to edit page when event is clicked', async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Team Meeting')).toBeInTheDocument()
    })

    const eventItem = screen.getByRole('button', { name: /Edit event/i })
    fireEvent.click(eventItem)

    expect(mockPush).toHaveBeenCalledWith('/event-detail?id=1')
  })
})
