import { render, screen, fireEvent } from '@testing-library/react'
import CreateEventPage from './page'
import { describe, it, expect, vi } from 'vitest'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const mockBack = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    back: mockBack,
    push: vi.fn(),
    refresh: vi.fn(),
  }),
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
      <CreateEventPage />
    </QueryClientProvider>
  )
}

describe('CreateEventPage', () => {
  it('renders correctly', () => {
    renderPage()
    expect(screen.getByText('New Event')).toBeInTheDocument()
  })

  it('navigates back on close', () => {
    renderPage()
    fireEvent.click(screen.getAllByRole('button', { name: 'Cancel' })[0])
    expect(mockBack).toHaveBeenCalled()
  })
})
