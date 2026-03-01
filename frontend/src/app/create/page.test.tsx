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

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
    },
  },
})

const renderWithQueryClient = (ui: React.ReactElement) => {
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  )
}

describe('CreateEventPage', () => {
  it('renders correctly', () => {
    renderWithQueryClient(<CreateEventPage />)
    expect(screen.getByText('New Event')).toBeInTheDocument()
  })

  it('navigates back on close', () => {
    renderWithQueryClient(<CreateEventPage />)
    const cancelButtons = screen.getAllByText('Cancel')
    fireEvent.click(cancelButtons[0])
    expect(mockBack).toHaveBeenCalled()
  })
})
