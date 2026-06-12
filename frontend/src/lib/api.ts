import {
  Event,
  EventStatus,
  CreateEventRequest,
  UpdateEventRequest,
  EventTimingRequest,
  EventResponse,
  MeResponse,
  CalendarInfo,
  DeviceListItem,
  DevicePasswordResponse,
} from '@/types/schema'

export type {
  Event,
  EventStatus,
  CreateEventRequest,
  UpdateEventRequest,
  EventTimingRequest,
  EventResponse,
  MeResponse,
  CalendarInfo,
  DeviceListItem,
  DevicePasswordResponse,
}

export type Timezone = string

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || '/api'

function getTelegramInitData(): string | null {
  if (typeof window === 'undefined') {
    return null
  }

  return window.Telegram?.WebApp?.initData || null
}

class ApiClient {
  private async request<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<T> {
    const url = `${API_BASE_URL}${endpoint}`
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options?.headers as Record<string, string>),
    }

    const initData = getTelegramInitData()
    if (initData) {
      headers['Authorization'] = `tma ${initData}`
    }

    const response = await fetch(url, { ...options, headers })

    if (!response.ok) {
      throw new Error(`API Error: ${response.status} ${response.statusText}`)
    }

    if (response.status === 204) {
      return {} as T
    }

    return response.json()
  }

  async getMe(): Promise<MeResponse> {
    return this.request<MeResponse>('/me')
  }

  async getEvents(): Promise<EventResponse[]> {
    return this.request<EventResponse[]>('/events')
  }

  async getEvent(id: string): Promise<EventResponse> {
    return this.request<EventResponse>(`/events/${id}`)
  }

  async createEvent(data: CreateEventRequest): Promise<EventResponse> {
    return this.request<EventResponse>('/events', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async updateEvent(
    id: string,
    data: UpdateEventRequest
  ): Promise<EventResponse> {
    return this.request<EventResponse>(`/events/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  }

  async deleteEvent(id: string): Promise<void> {
    return this.request<void>(`/events/${id}`, {
      method: 'DELETE',
    })
  }

  async getDevices(): Promise<DeviceListItem[]> {
    return this.request<DeviceListItem[]>('/devices')
  }

  async createDevice(name: string): Promise<DevicePasswordResponse> {
    return this.request<DevicePasswordResponse>('/devices', {
      method: 'POST',
      body: JSON.stringify({ name }),
    })
  }

  async deleteDevice(id: string): Promise<void> {
    return this.request<void>(`/devices/${id}`, {
      method: 'DELETE',
    })
  }
}

export const api = new ApiClient()
