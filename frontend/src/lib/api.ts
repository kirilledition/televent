import {
  Event,
  EventStatus,
  CreateEventRequest,
  UpdateEventRequest,
  EventResponse,
  User,
  DeviceListItem,
  DevicePasswordResponse,
} from '@/types/schema'

export type {
  Event,
  EventStatus,
  CreateEventRequest,
  UpdateEventRequest,
  EventResponse,
  User,
  DeviceListItem,
  DevicePasswordResponse,
}

export type Timezone = string

const API_BASE_url =
  process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001/api' // Adjust as needed

class ApiClient {
  private async request<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<T> {
    const url = `${API_BASE_url}${endpoint}`
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options?.headers as Record<string, string>),
    }

    // Inject fake auth for local dev
    if (process.env.NODE_ENV === 'development') {
      headers['Authorization'] =
        'tma auth_date=1700000000&query_id=AAGyswdAAAAAAALLB0A&user=%7B%22id%22%3A123456789%2C%22first_name%22%3A%22Test%22%2C%22last_name%22%3A%22User%22%2C%22username%22%3A%22testuser%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Afalse%2C%22allows_write_to_pm%22%3Atrue%7D&hash=075e0d126e8e57060d9fdca6599f95482a4fdb97521e1a937f7c5dd8f6190719'
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

  async getMe(): Promise<User> {
    return this.request<User>('/me')
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
