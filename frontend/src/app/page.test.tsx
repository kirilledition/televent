import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import CalendarPage from './page'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { api, EventResponse } from '@/lib/api'

// Mock useRouter
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

// Mock API
vi.mock('@/lib/api', () => ({
  api: {
    getEvents: vi.fn(),
    deleteEvent: vi.fn(),
  },
}))

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

describe('CalendarPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(api.getEvents).mockResolvedValue([
      {
        id: '1',
        summary: 'Team Meeting',
        start: '2023-10-01T10:00:00',
        end: '2023-10-01T11:00:00',
        is_all_day: false,
        timezone: 'UTC',
        version: 1,
        etag: '1',
        created_at: '2023-09-01T00:00:00Z',
        updated_at: '2023-09-01T00:00:00Z',
        status: 'Confirmed',
        uid: 'uid-1',
        user_id: 'u1',
      } as unknown as EventResponse,
    ])
  })

  it('renders correctly with dummy events', async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )
    expect(await screen.findByText('Calendar')).toBeInTheDocument()
    expect(screen.getByText('New event')).toBeInTheDocument()

    // Wait for events to load
    await waitFor(() => {
      expect(screen.getByText('Team Meeting')).toBeInTheDocument()
    })
  })

  it('navigates to create page on button click', async () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CalendarPage />
      </QueryClientProvider>
    )
    await waitFor(() => screen.getByText('Team Meeting'))

    fireEvent.click(screen.getByText('New event'))
    expect(mockPush).toHaveBeenCalledWith('/create')
  })

  it('deletes an event locally', async () => {
    vi.mocked(api.deleteEvent).mockResolvedValue({})
    const queryClient = createQueryClient()

    render(
      <QueryClientProvider client={queryClient}>
        <CalendarPage />
      </QueryClientProvider>
    )

    await waitFor(() => screen.getByText('Team Meeting'))

    // The component uses AlertDialog, not window.confirm
    // We need to trigger delete, then confirm in dialog
    const deleteBtns = screen.getAllByRole('button', { name: /Delete event/i })
    fireEvent.click(deleteBtns[0])

    // Find confirm button in dialog
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

    await waitFor(() => screen.getByText('Team Meeting'))

    const eventItems = screen.getAllByRole('button', { name: /Edit event/i })
    fireEvent.click(eventItems[0])
    expect(mockPush).toHaveBeenCalledWith('/event-detail?id=1')
  })
})
