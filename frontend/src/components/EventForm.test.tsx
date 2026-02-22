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
  const actual = await importOriginal()
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
    // Verify autoFocus
    expect(screen.getByLabelText(/Title/i)).toHaveFocus()
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

  it('submits on Ctrl+Enter', async () => {
    ;(api.createEvent as any).mockResolvedValue({})

    render(
      <QueryClientProvider client={createQueryClient()}>
        <EventForm />
      </QueryClientProvider>
    )

    fireEvent.change(screen.getByLabelText(/Title/i), {
      target: { value: 'Keyboard Shortcut Event' },
    })

    // Simulate Ctrl+Enter on the form
    const form = screen.getByLabelText(/Title/i).closest('form')!
    fireEvent.keyDown(form, { key: 'Enter', ctrlKey: true })

    await waitFor(() => {
      expect(api.createEvent).toHaveBeenCalledWith(
        expect.objectContaining({
          summary: 'Keyboard Shortcut Event',
        })
      )
    })
  })
})
