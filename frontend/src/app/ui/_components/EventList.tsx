'use client';

import { EventResponse } from '@/types/schema';
import { EventItem } from './EventItem';
import { parseISO, format, isSameDay, addDays } from 'date-fns';

interface EventListProps {
    events: EventResponse[];
    onDeleteEvent: (id: string) => void;
    onEditEvent: (event: EventResponse) => void;
}

export function EventList({ events, onDeleteEvent, onEditEvent }: EventListProps) {
    // Sort events by start time
    const sortedEvents = [...events].sort((a, b) => {
        return new Date(a.start).getTime() - new Date(b.start).getTime();
    });

    // Group events by date (YYYY-MM-DD)
    const groupedEvents = sortedEvents.reduce((acc, event) => {
        // Derive date key from start time
        const date = format(parseISO(event.start), 'yyyy-MM-dd');
        if (!acc[date]) {
            acc[date] = [];
        }
        acc[date].push(event);
        return acc;
    }, {} as Record<string, EventResponse[]>);

    const formatDateHeader = (dateStr: string) => {
        const date = new Date(dateStr + 'T00:00:00');
        const today = new Date();
        today.setHours(0, 0, 0, 0);

        if (isSameDay(date, today)) {
            return 'Today';
        }

        const tomorrow = addDays(today, 1);
        if (isSameDay(date, tomorrow)) {
            return 'Tomorrow';
        }

        return date.toLocaleDateString('en-US', {
            weekday: 'long',
            month: 'long',
            day: 'numeric',
            year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined
        });
    };

    if (sortedEvents.length === 0) {
        return (
            <div className="text-center py-16" style={{ color: 'var(--ctp-overlay0)' }}>
                <p className="text-sm">No events yet. Create your first event to get started.</p>
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {Object.entries(groupedEvents).sort().map(([date, dateEvents]) => (
                <div key={date}>
                    <div className="text-sm font-medium mb-3 px-2" style={{ color: 'var(--ctp-subtext0)' }}>
                        {formatDateHeader(date)}
                    </div>
                    <div className="space-y-0">
                        {dateEvents.map((event) => (
                            <EventItem
                                key={event.id}
                                event={event}
                                onDelete={onDeleteEvent}
                                onEdit={onEditEvent}
                            />
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
}
