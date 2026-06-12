import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import CalendarPage from './page'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api } from '@/lib/api'

// Mock useRouter
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

vi.mock('@/lib/api', () => ({
  api: {
    getEvents: vi.fn(),
    deleteEvent: vi.fn(),
  },
}))

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })

  return render(
    <QueryClientProvider client={queryClient}>
      <CalendarPage />
    </QueryClientProvider>
  )
}

const event = {
  id: '1',
  uid: 'uid-1',
  summary: 'Team Meeting',
  description: null,
  location: 'Conference Room',
  start: '2026-01-01T10:00:00Z',
  end: '2026-01-01T11:00:00Z',
  start_date: null,
  end_date: null,
  is_all_day: false,
  status: 'Confirmed',
  timezone: 'UTC',
  rrule: null,
}

describe('CalendarPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ;(api.getEvents as any).mockResolvedValue([event])
    ;(api.deleteEvent as any).mockResolvedValue(undefined)
  })

  it('renders correctly with API events', async () => {
    renderPage()
    expect(await screen.findByText('Calendar')).toBeInTheDocument()
    expect(await screen.findByText('New event')).toBeInTheDocument()
    expect(await screen.findByText('Team Meeting')).toBeInTheDocument()
  })

  it('navigates to create page on button click', async () => {
    renderPage()
    fireEvent.click(await screen.findByText('New event'))
    expect(mockPush).toHaveBeenCalledWith('/create')
  })

  it('deletes an event via API', async () => {
    renderPage()

    const deleteBtn = await screen.findByRole('button', {
      name: /Delete event: Team Meeting/i,
    })

    fireEvent.click(deleteBtn)
    fireEvent.click(screen.getByRole('button', { name: 'Delete' }))

    await waitFor(() => {
      expect(api.deleteEvent).toHaveBeenCalledWith('1')
    })
  })

  it('navigates to detail page when event is clicked', async () => {
    renderPage()
    const eventItem = await screen.findByRole('button', {
      name: /Edit event: Team Meeting/i,
    })
    fireEvent.click(eventItem)
    expect(mockPush).toHaveBeenCalledWith('/event-detail?id=1')
  })
})
