import type { Event } from './api';

/**
 * Get the event's start date/time for sorting and grouping.
 * Handles both timed events (start) and all-day events (start_date).
 */
function getEventDateTime(event: Event): Date {
    if (event.start) {
        return new Date(event.start);
    } else if (event.start_date) {
        return new Date(event.start_date);
    }
    // Fallback to current time if neither is set (shouldn't happen with valid data)
    return new Date();
}

export function groupEventsByDate(events: Event[]): Record<string, Event[]> {
    const grouped: Record<string, Event[]> = {};

    events.forEach(event => {
        const date = getEventDateTime(event);
        const dateKey = date.toLocaleDateString(undefined, {
            weekday: 'long',
            year: 'numeric',
            month: 'long',
            day: 'numeric'
        });

        if (!grouped[dateKey]) {
            grouped[dateKey] = [];
        }
        grouped[dateKey].push(event);
    });

    // Sort events within each group by time
    Object.values(grouped).forEach(group => {
        group.sort((a, b) => getEventDateTime(a).getTime() - getEventDateTime(b).getTime());
    });

    return grouped;
}

export function groupEventsByDateEntries(events: Event[]): [string, Event[]][] {
    const grouped = groupEventsByDate(events);
    // Sort entries by the first event's date in each group
    return Object.entries(grouped).sort((a, b) => {
        return getEventDateTime(a[1][0]).getTime() - getEventDateTime(b[1][0]).getTime();
    });
}
