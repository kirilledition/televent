import { retrieveLaunchParams } from '@telegram-apps/sdk-react';

// Backend is served at root, not /api
const API_BASE = '';

/**
 * API client for Televent backend.
 * Automatically attaches Telegram initData to requests for authentication.
 */

interface FetchOptions extends Omit<RequestInit, 'headers'> {
    headers?: Record<string, string>;
}

// Event status matches backend enum
export type EventStatus = 'Confirmed' | 'Tentative' | 'Cancelled';

// Event entity - matches backend EventResponse
export interface Event {
    id: string;
    user_id: string;
    uid: string;
    summary: string;
    description?: string;
    location?: string;
    start?: string; // ISO string for timed events
    end?: string; // ISO string for timed events
    start_date?: string; // Date string for all-day events
    end_date?: string; // Date string for all-day events
    is_all_day: boolean;
    status: EventStatus;
    rrule?: string;
    timezone: string;
    version: number;
    etag: string;
    created_at: string;
    updated_at: string;
}

// Calendar entity - matches backend CalendarInfo
export interface Calendar {
    id: string;
    name: string;
    color: string;
    sync_token: string;
}

// Request Types
export interface CreateEventRequest {
    uid: string;
    summary: string;
    description?: string;
    location?: string;
    start: string; // ISO string
    end: string; // ISO string
    is_all_day: boolean;
    timezone: string;
    rrule?: string;
}

export interface UpdateEventRequest {
    summary?: string;
    description?: string;
    location?: string;
    start?: string;
    end?: string;
    is_all_day?: boolean;
    status?: EventStatus;
    rrule?: string;
}

export interface ListEventsQuery {
    start?: string;
    end?: string;
    limit?: number;
    offset?: number;
}

export interface CreateDeviceRequest {
    name: string;
}

// Response Types
export interface DevicePasswordResponse {
    id: string;
    name: string;
    password?: string;
    created_at: string;
    last_used_at?: string;
}

export interface DeviceListItem {
    id: string;
    name: string;
    created_at: string;
    last_used_at?: string;
}

export interface User {
    id: string;
    username: string | null;
    authenticated: boolean;
    timezone: string;
}

async function fetchApi<T>(path: string, options: FetchOptions = {}): Promise<T> {
    let initData: string | undefined;

    try {
        const { initDataRaw } = retrieveLaunchParams();
        initData = (initDataRaw as string) || '';
    } catch {
        // Not in Telegram environment - initData won't be available
        // console.warn('Could not retrieve Telegram initData');
    }

    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        ...options.headers,
    };

    if (initData) {
        headers['Authorization'] = `tma ${initData}`;
    }

    const response = await fetch(`${API_BASE}${path}`, {
        ...options,
        headers,
    });

    if (!response.ok) {
        const errorText = await response.text().catch(() => 'Unknown error');
        throw new Error(`API error ${response.status}: ${errorText}`);
    }

    return response.json();
}

/**
 * Typed API client with methods for each endpoint.
 */
export const api = {
    /**
     * Get the current authenticated user's profile.
     */
    getMe: () => fetchApi<User>('/me'),

    /**
     * User's Calendars
     */
    getCalendars: () => fetchApi<Calendar[]>('/calendars'),

    /**
     * Events - all operations are user-scoped (no calendar_id needed)
     */
    getEvents: (query?: ListEventsQuery) => {
        const params = new URLSearchParams();
        if (query?.start) params.append('start', query.start);
        if (query?.end) params.append('end', query.end);
        if (query?.limit) params.append('limit', query.limit.toString());
        if (query?.offset) params.append('offset', query.offset.toString());

        const queryString = params.toString();
        return fetchApi<Event[]>(`/events${queryString ? '?' + queryString : ''}`);
    },

    getEvent: (id: string) => fetchApi<Event>(`/events/${id}`),

    createEvent: (data: CreateEventRequest) =>
        fetchApi<Event>('/events', {
            method: 'POST',
            body: JSON.stringify(data),
        }),

    updateEvent: (id: string, data: UpdateEventRequest) =>
        fetchApi<Event>(`/events/${id}`, {
            method: 'PUT',
            body: JSON.stringify(data),
        }),

    deleteEvent: (id: string) =>
        fetchApi<void>(`/events/${id}`, {
            method: 'DELETE',
        }),

    /**
     * Devices - simplified API (no userId needed, uses authenticated user)
     */
    getDevices: () => fetchApi<DeviceListItem[]>('/devices'),

    createDevice: (name: string) =>
        fetchApi<DevicePasswordResponse>('/devices', {
            method: 'POST',
            body: JSON.stringify({ name }),
        }),

    deleteDevice: (deviceId: string) =>
        fetchApi<void>(`/devices/${deviceId}`, {
            method: 'DELETE',
        }),

    /**
     * Health check endpoint.
     */
    health: () => fetchApi<{ status: string }>('/health'),
};

export default api;
