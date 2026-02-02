'use client';

import { useState } from 'react';
import { EventResponse } from '@/types/schema';
import { Trash2, MapPin, Clock, Check, X } from 'lucide-react';
import { format, differenceInMinutes, parseISO } from 'date-fns';

interface EventItemProps {
    event: EventResponse;
    onDelete: (id: string) => void;
    onEdit: (event: EventResponse) => void;
}

export function EventItem({ event, onDelete, onEdit }: EventItemProps) {
    const [isConfirming, setIsConfirming] = useState(false);
    // Handle all-day events where start/end might be null but start_date/end_date exist
    const start = parseISO(event.start || event.start_date);
    const end = parseISO(event.end || event.end_date);
    const duration = differenceInMinutes(end, start);

    const formatDuration = (minutes: number) => {
        const hours = Math.floor(minutes / 60);
        const mins = minutes % 60;
        let result = '';
        if (hours > 0) {
            result += `${hours}h`;
            if (mins > 0) result += ` ${mins}m`;
        } else if (mins > 0) {
            result = `${mins}m`;
        }
        return result;
    };

    return (
        <div className="group relative flex items-start gap-3 px-4 py-4 rounded-lg mb-2 transition-colors hover:opacity-90" style={{ backgroundColor: 'var(--ctp-mantle)' }}>
            {/* Sapphire indicator */}
            <div className="w-1 h-full rounded-full absolute left-0 top-0 bottom-0" style={{ backgroundColor: 'var(--ctp-sapphire)' }} />

            {/* Event content */}
            <div
                className="flex-1 min-w-0 pl-2 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ctp-sapphire)] rounded-sm"
                onClick={() => onEdit(event)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        onEdit(event);
                    }
                }}
            >
                <div className="flex items-start justify-between gap-3 mb-2">
                    <h3 className="text-lg font-medium" style={{ color: 'var(--ctp-text)' }}>{event.summary}</h3>

                    {/* Delete button - always visible on mobile */}
                    {isConfirming ? (
                        <div className="flex gap-2">
                            <button
                                onClick={(e) => {
                                    e.stopPropagation();
                                    onDelete(event.id);
                                }}
                                className="p-2 rounded-lg transition-all hover:opacity-70"
                                style={{ backgroundColor: 'var(--ctp-surface0)' }}
                                aria-label="Confirm delete"
                            >
                                <Check className="w-4 h-4" style={{ color: 'var(--ctp-red)' }} />
                            </button>
                            <button
                                onClick={(e) => {
                                    e.stopPropagation();
                                    setIsConfirming(false);
                                }}
                                className="p-2 rounded-lg transition-all hover:opacity-70"
                                style={{ backgroundColor: 'var(--ctp-surface0)' }}
                                aria-label="Cancel delete"
                            >
                                <X className="w-4 h-4" style={{ color: 'var(--ctp-subtext0)' }} />
                            </button>
                        </div>
                    ) : (
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                setIsConfirming(true);
                            }}
                            className="p-2 rounded-lg transition-all hover:opacity-70"
                            style={{ backgroundColor: 'var(--ctp-surface0)' }}
                            aria-label="Delete event"
                        >
                            <Trash2 className="w-4 h-4" style={{ color: 'var(--ctp-subtext0)' }} />
                        </button>
                    )}
                </div>

                <div className="space-y-1">
                    {!event.is_all_day ? (
                        <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--ctp-subtext1)' }}>
                            <Clock className="w-4 h-4" />
                            <span>{format(start, 'HH:mm')}</span>
                            {duration > 0 && (
                                <span style={{ color: 'var(--ctp-overlay1)' }}>
                                    • {formatDuration(duration)}
                                </span>
                            )}
                        </div>
                    ) : (
                        <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--ctp-subtext1)' }}>
                            <Clock className="w-4 h-4" />
                            <span>All Day</span>
                        </div>
                    )}

                    {event.location && (
                        <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--ctp-subtext1)' }}>
                            <MapPin className="w-4 h-4" />
                            <span>{event.location}</span>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
