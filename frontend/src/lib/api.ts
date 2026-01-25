import { retrieveLaunchParams } from '@telegram-apps/sdk-react';
import type { Calendar, Event, EventStatus } from '@/types/schema';

const API_BASE = '/api';

/**
 * API client for Televent backend.
 * Automatically attaches Telegram initData to requests for authentication.
 */

interface FetchOptions extends Omit<RequestInit, 'headers'> {
    headers?: Record<string, string>;
}

// Re-export types from schema
export type { Calendar, Event, EventStatus };

// Request Types
export interface CreateEventRequest {
    calendar_id: string; // Uuid
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
    calendar_id: string;
    start?: string;
    end?: string;
    limit?: number;
    offset?: number;
}

export interface CreateDeviceRequest {
    name: string;
}

// Response Types matches Schema but we might need specific ones like DevicePasswordResponse
export interface DevicePasswordResponse {
    id: string; // Uuid
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
    telegram_id: number;
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
     * Events
     */
    getEvents: (query: ListEventsQuery) => {
        const params = new URLSearchParams();
        params.append('calendar_id', query.calendar_id);
        if (query.start) params.append('start', query.start);
        if (query.end) params.append('end', query.end);
        if (query.limit) params.append('limit', query.limit.toString());
        if (query.offset) params.append('offset', query.offset.toString());

        return fetchApi<Event[]>(`/events?${params.toString()}`);
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
     * Devices
     */
    getDevices: (userId: string) => fetchApi<DeviceListItem[]>(`/users/${userId}/devices`),

    createDevice: (userId: string, name: string) =>
        fetchApi<DevicePasswordResponse>(`/users/${userId}/devices`, {
            method: 'POST',
            body: JSON.stringify({ name }),
        }),

    deleteDevice: (userId: string, deviceId: string) =>
        fetchApi<void>(`/users/${userId}/devices/${deviceId}`, {
            method: 'DELETE',
        }),

    /**
     * Health check endpoint.
     */
    health: () => fetchApi<{ status: string }>('/health'),
};

export default api;
