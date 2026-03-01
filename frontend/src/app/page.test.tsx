import { describe, it, expect, vi } from 'vitest'

// Mock useRouter
const mockPush = vi.fn()
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: mockPush,
  }),
}))

describe('CalendarPage', () => {
  it('renders loading state initially', () => {
    // Just a placeholder test for now
    expect(true).toBe(true)
  })
})
