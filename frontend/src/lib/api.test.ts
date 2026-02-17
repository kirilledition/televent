import { describe, it, expect, vi, beforeEach } from 'vitest'
import { api } from './api'

global.fetch = vi.fn()

describe('API Client', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches events', async () => {
    const mockEvents = [{ id: '1', title: 'Test Event' }]
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockEvents,
    })

    const events = await api.getEvents()
    expect(events).toEqual(mockEvents)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/events'),
      expect.any(Object)
    )
  })

  it('gets single event', async () => {
    const mockEvent = { id: '1', title: 'Test Event' }
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockEvent,
    })

    const event = await api.getEvent('1')
    expect(event).toEqual(mockEvent)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/events/1'),
      expect.any(Object)
    )
  })

  it('handles errors', async () => {
    ;(global.fetch as any).mockResolvedValue({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
    })

    await expect(api.getEvents()).rejects.toThrow(
      'API Error: 500 Internal Server Error'
    )
  })

  it('creates event', async () => {
    const newEvent = { title: 'New Event' }
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => newEvent,
    })

    const result = await api.createEvent(newEvent as any)
    expect(result).toEqual(newEvent)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/events'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify(newEvent),
      })
    )
  })

  it('updates event', async () => {
    const updateData = { title: 'Updated Event' }
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => updateData,
    })

    const result = await api.updateEvent('1', updateData as any)
    expect(result).toEqual(updateData)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/events/1'),
      expect.objectContaining({
        method: 'PUT',
        body: JSON.stringify(updateData),
      })
    )
  })

  it('deletes event (204 No Content)', async () => {
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      status: 204,
    })

    await api.deleteEvent('1')
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/events/1'),
      expect.objectContaining({ method: 'DELETE' })
    )
  })

  it('gets me (user info)', async () => {
    const mockUser = { id: 'u1' }
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockUser,
    })
    const user = await api.getMe()
    expect(user).toEqual(mockUser)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/me'),
      expect.any(Object)
    )
  })

  it('gets devices', async () => {
    const mockDevices = [{ id: 'd1' }]
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockDevices,
    })
    const devices = await api.getDevices()
    expect(devices).toEqual(mockDevices)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/devices'),
      expect.any(Object)
    )
  })

  it('creates device', async () => {
    const mockDevice = { id: 'd1', password: 'pw' }
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockDevice,
    })
    const device = await api.createDevice('My Device')
    expect(device).toEqual(mockDevice)
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/devices'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ name: 'My Device' }),
      })
    )
  })

  it('deletes device', async () => {
    ;(global.fetch as any).mockResolvedValue({
      ok: true,
      status: 204,
    })
    await api.deleteDevice('d1')
    expect(global.fetch).toHaveBeenCalledWith(
      expect.stringContaining('/devices/d1'),
      expect.objectContaining({ method: 'DELETE' })
    )
  })
})
