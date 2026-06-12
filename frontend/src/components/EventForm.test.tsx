import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { EventForm } from './EventForm'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { api } from '@/lib/api'

// Mock next/navigation
const mockPush = vi.fn()
const mockRefresh = vi.fn()
const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
    refresh: mockRefresh,
    back: mockBack,
  }),
}))

// Mock API
vi.mock('@/lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api')>()
  return {
    ...actual,
    api: {
      createEvent: vi.fn(),
      updateEvent: vi.fn(),
    },
  }
})

// Setup QueryClient
const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

describe('EventForm', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Reset crypto.randomUUID
    Object.defineProperty(global, 'crypto', {
      value: {
        randomUUID: () => 'test-uuid',
      },
      writable: true,
    })
  })

  it('renders create form correctly', () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <EventForm />
      </QueryClientProvider>
    )

    expect(screen.getByLabelText(/Title/i)).toBeInTheDocument()
    expect(screen.getByText('Create Event')).toBeInTheDocument()
  })

  it('submits new event', async () => {
    ;(api.createEvent as any).mockResolvedValue({})

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EventForm />
      </QueryClientProvider>
    )

    fireEvent.change(screen.getByLabelText(/Title/i), {
      target: { value: 'New Meeting' },
    })
    // Fill required fields if any (Start is usually prefilled)

    fireEvent.click(screen.getByText('Create Event'))

    await waitFor(() => {
      expect(api.createEvent).toHaveBeenCalledWith(
        expect.objectContaining({
          summary: 'New Meeting',
          uid: 'test-uuid',
        })
      )
    })
  })

  it('submits update event', async () => {
    const initialData = {
      id: '1',
      summary: 'Existing Event',
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
    } as any

    ;(api.updateEvent as any).mockResolvedValue({})

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EventForm initialData={initialData} isEditing={true} />
      </QueryClientProvider>
    )

    expect(screen.getByDisplayValue('Existing Event')).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText(/Title/i), {
      target: { value: 'Updated Event' },
    })
    fireEvent.click(screen.getByText('Update Event'))

    await waitFor(() => {
      expect(api.updateEvent).toHaveBeenCalledWith(
        '1',
        expect.objectContaining({
          summary: 'Updated Event',
        })
      )
    })
  })

  it('preserves an existing multi-day all-day span when updating', async () => {
    const initialData = {
      id: '1',
      summary: 'Company Offsite',
      description: null,
      location: null,
      start: null,
      end: null,
      start_date: '2023-10-01',
      end_date: '2023-10-04',
      is_all_day: true,
      timezone: 'UTC',
      status: 'Confirmed',
      uid: 'uid-1',
      rrule: null,
    } as any

    ;(api.updateEvent as any).mockResolvedValue({})

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EventForm initialData={initialData} isEditing={true} />
      </QueryClientProvider>
    )

    fireEvent.click(screen.getByText('Update Event'))

    await waitFor(() => {
      expect(api.updateEvent).toHaveBeenCalledWith(
        '1',
        expect.objectContaining({
          timing: {
            kind: 'all_day',
            start_date: '2023-10-01',
            end_date: '2023-10-04',
          },
        })
      )
    })
  })
})
