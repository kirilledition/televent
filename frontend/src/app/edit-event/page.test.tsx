import { render, screen, waitFor } from '@testing-library/react'
import EditEventPage from './page'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { api } from '@/lib/api'
import { useSearchParams } from 'next/navigation'

// Mock next/navigation
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: vi.fn(),
    push: vi.fn(),
  }),
  useSearchParams: vi.fn(),
}))

// Mock API
vi.mock('@/lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api')>()
  return {
    ...actual,
    api: {
      getEvent: vi.fn(),
      updateEvent: vi.fn(),
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

describe('EditEventPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders invalid ID message when no ID', () => {
    ;(useSearchParams as any).mockReturnValue({
      get: () => null,
    })

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EditEventPage />
      </QueryClientProvider>
    )
    expect(screen.getByText('Invalid Event ID')).toBeInTheDocument()
  })

  it('fetches and renders event form when ID is present', async () => {
    ;(useSearchParams as any).mockReturnValue({
      get: (key: string) => (key === 'id' ? '1' : null),
    }) // Added semicolon

    const mockEvent = {
      id: '1',
      summary: 'Test Event',
      description: null,
      location: null,
      start: '2023-10-01T10:00:00',
      end: '2023-10-01T11:00:00',
      start_date: null,
      end_date: null,
      is_all_day: false,
      timezone: 'UTC',
      status: 'Confirmed',
      uid: 'uid-1',
      rrule: null,
    }

    ;(api.getEvent as any).mockResolvedValue(mockEvent)

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EditEventPage />
      </QueryClientProvider>
    )

    expect(screen.getByText('Loading...')).toBeInTheDocument()

    await waitFor(() => {
      expect(screen.getByDisplayValue('Test Event')).toBeInTheDocument()
    })
  })

  it('renders error state', async () => {
    ;(useSearchParams as any).mockReturnValue({
      get: (key: string) => (key === 'id' ? '1' : null),
    }) // Added semicolon
    ;(api.getEvent as any).mockRejectedValue(new Error('Failed to fetch'))

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EditEventPage />
      </QueryClientProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Error loading event')).toBeInTheDocument()
    })
  })
})
