import type { Event } from './api';

export function groupEventsByDate(events: Event[]): Record<string, Event[]> {
    const grouped: Record<string, Event[]> = {};

    events.forEach(event => {
        const date = new Date(event.start);
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
        group.sort((a, b) => new Date(a.start).getTime() - new Date(b.start).getTime());
    });

    // keys are not necessarily sorted, but we can return object or array of entries if we need order.
    // For now returning helper that handles keys sorting? 
    // Actually returning entries is better for iterating in React.
    return grouped;
}

export function groupEventsByDateEntries(events: Event[]): [string, Event[]][] {
    const grouped = groupEventsByDate(events);
    // Sort keys (dates)
    return Object.entries(grouped).sort((a, b) => {
        // We need to parse the date key back to sort, which is hard with localized string.
        // Better approach: use ISO date as key for sorting, and formatted date for display.

        // Let's rewrite this to be more robust.
        // We will just pick the first event's start time as the sort key for the group.
        return new Date(a[1][0].start).getTime() - new Date(b[1][0].start).getTime();
    });
}
