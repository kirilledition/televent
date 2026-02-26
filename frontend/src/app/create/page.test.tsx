import { render, screen, fireEvent } from '@testing-library/react'
import CreateEventPage from './page'
import { describe, it, expect, vi } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
  }),
}))

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

describe('CreateEventPage', () => {
  it('renders correctly', () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CreateEventPage />
      </QueryClientProvider>
    )
    expect(screen.getByText('New Event')).toBeInTheDocument()
  })

  it('navigates back on close', () => {
    render(
      <QueryClientProvider client={createQueryClient()}>
        <CreateEventPage />
      </QueryClientProvider>
    )
    // Use getAllByText because Cancel appears in both the header and the form button
    const cancelButtons = screen.getAllByText('Cancel')
    fireEvent.click(cancelButtons[0])
    expect(mockBack).toHaveBeenCalled()
  })
})
