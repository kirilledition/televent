'use client';

import { useMemo, memo } from 'react';
import { EventResponse } from '@/types/schema';
import { EventItem } from './EventItem';
import { parseISO, format, isSameDay, addDays } from 'date-fns';
import { Plus, Calendar } from 'lucide-react';

interface EventListProps {
    events: EventResponse[];
    onDeleteEvent: (id: string) => void;
    onEditEvent: (event: EventResponse) => void;
    onCreateEvent: () => void;
}

export const EventList = memo(function EventList({ events, onDeleteEvent, onEditEvent, onCreateEvent }: EventListProps) {
    // Sort events by start time
    const sortedEvents = useMemo(() => {
        // Optimization: Map to timestamp first to avoid parsing dates N*logN times during sort
        const eventsWithTime = events.map((e) => {
            // Handle potentially missing start (e.g. all-day events might have null start but have start_date)
            // Note: EventResponse type defines start as string, but runtime it might be null for all-day events
            const timeStr = e.start || e.start_date;
            return {
                event: e,
                time: timeStr ? new Date(timeStr).getTime() : 0,
            };
        });

        eventsWithTime.sort((a, b) => a.time - b.time);

        return eventsWithTime.map((wrapper) => wrapper.event);
    }, [events]);

    // Group events by date (YYYY-MM-DD)
    const groupedEvents = useMemo(() => {
        return sortedEvents.reduce((acc, event) => {
            // Derive date key from start time
            // Fallback to start_date if start is missing (all-day events)
            const dateStr = event.start || event.start_date;
            if (!dateStr) return acc;

            // Optimization: For all-day events, start_date is already YYYY-MM-DD
            // This avoids expensive parseISO + format calls
            const date = (event.is_all_day && event.start_date)
                ? event.start_date
                : format(parseISO(dateStr), 'yyyy-MM-dd');

            if (!acc[date]) {
                acc[date] = [];
            }
            acc[date].push(event);
            return acc;
        }, {} as Record<string, EventResponse[]>);
    }, [sortedEvents]);

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

    // Memoize sorted entries to prevent sorting on every render
    const sortedGroupedEvents = useMemo(() => {
        return Object.entries(groupedEvents).sort();
    }, [groupedEvents]);

    if (sortedEvents.length === 0) {
        return (
            <div className="text-center py-16 flex flex-col items-center justify-center space-y-4" style={{ color: 'var(--ctp-overlay0)' }}>
                <Calendar className="w-12 h-12 opacity-50 mb-2" />
                <div className="space-y-1">
                    <p className="font-medium text-lg" style={{ color: 'var(--ctp-text)' }}>No events yet</p>
                    <p className="text-sm">Create your first event to get started.</p>
                </div>
                <button
                    onClick={onCreateEvent}
                    className="flex items-center gap-2 px-4 py-2 mt-2 text-sm font-medium rounded-lg transition-colors hover:opacity-90"
                    style={{ backgroundColor: 'var(--ctp-surface0)', color: 'var(--ctp-text)' }}
                >
                    <Plus className="w-4 h-4" />
                    Create Event
                </button>
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {sortedGroupedEvents.map(([date, dateEvents]) => (
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
});
