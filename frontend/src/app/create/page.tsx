'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import {
    List,
    Section,
    Input,
    Textarea,
    Cell,
    Spinner,
    Text,
    Switch,
} from '@telegram-apps/telegram-ui';
import { mainButton, backButton, hapticFeedback } from '@telegram-apps/sdk-react';
import useSWR, { mutate } from 'swr';
import { api, User } from '@/lib/api';

export default function CreateEventPage() {
    const router = useRouter();

    const [summary, setSummary] = useState('');
    const [description, setDescription] = useState('');
    const [location, setLocation] = useState('');
    const [start, setStart] = useState(() => {
        const now = new Date();
        now.setMinutes(0, 0, 0);
        now.setHours(now.getHours() + 1);
        try {
            return now.toISOString().slice(0, 16);
        } catch {
            return '';
        }
    });
    const [end, setEnd] = useState(() => {
        const now = new Date();
        now.setMinutes(0, 0, 0);
        now.setHours(now.getHours() + 2);
        try {
            return now.toISOString().slice(0, 16);
        } catch {
            return '';
        }
    });

    const [isAllDay, setIsAllDay] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Fetch user (no need for calendars - events are user-scoped)
    const { data: user, isLoading: userLoading } = useSWR<User, Error>('user', api.getMe);

    // Handle Save
    const handleSave = useCallback(async () => {
        if (!user) {
            setError("User not loaded");
            return;
        }

        // Validation
        if (!summary.trim()) {
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError("Event title is required");
            return;
        }

        if (!start) {
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError("Start time is required");
            return;
        }

        if (!end) {
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError("End time is required");
            return;
        }

        // Validate that start is before end
        const startDate = new Date(start);
        const endDate = new Date(end);
        if (startDate >= endDate) {
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError("Start time must be before end time");
            return;
        }

        setIsSubmitting(true);
        setError(null);

        try {
            await api.createEvent({
                uid: crypto.randomUUID(),
                summary: summary.trim(),
                description: description.trim() || undefined,
                location: location.trim() || undefined,
                start: startDate.toISOString(),
                end: endDate.toISOString(),
                is_all_day: isAllDay,
                timezone: user.timezone || 'UTC',
                rrule: undefined,
            });

            try { hapticFeedback.notificationOccurred('success'); } catch { }

            // Invalidate events cache to refresh the list
            mutate('events');

            router.back();
        } catch (err) {
            console.error(err);
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError(err instanceof Error ? err.message : 'Failed to create event');
            setIsSubmitting(false);
        }
    }, [user, summary, description, location, start, end, isAllDay, router]);

    // Setup Buttons
    useEffect(() => {
        // Mount logic (ensure components are mounted)
        try {
            if (!mainButton.isMounted()) mainButton.mount();
            if (!backButton.isMounted()) backButton.mount();
        } catch {
            // Ignore if not supported
        }
    }, []);

    useEffect(() => {
        try {
            if (!mainButton.isMounted() || !backButton.isMounted()) return;

            mainButton.setParams({
                text: isSubmitting ? 'SAVING...' : 'SAVE',
                isVisible: true,
                isEnabled: !isSubmitting,

            });

            backButton.show();

            const onMainClick = () => handleSave();
            const onBackClick = () => router.back();

            const cleanupMain = mainButton.onClick(onMainClick);
            const cleanupBack = backButton.onClick(onBackClick);

            return () => {
                cleanupMain();
                cleanupBack();
                try {
                    mainButton.setParams({ isVisible: false });
                    backButton.hide();
                } catch { }
            };
        } catch {
            return;
        }

    }, [isSubmitting, handleSave, router]);

    if (userLoading) {
        return (
            <div style={{
                display: 'flex',
                justifyContent: 'center',
                alignItems: 'center',
                minHeight: '100vh',
                background: 'var(--ctp-base)'
            }}>
                <Spinner size="l" />
            </div>
        );
    }

    return (
        <div style={{ background: 'var(--ctp-base)', minHeight: '100vh' }}>
            <List>
                {error && (
                    <Section>
                        <Cell>
                            <Text style={{
                                color: 'var(--ctp-red)',
                                fontWeight: 600
                            }}>
                                {error}
                            </Text>
                        </Cell>
                    </Section>
                )}

                <Section header="Event Details">
                    <Input
                        header="Title"
                        placeholder="Team Meeting, Lunch, etc."
                        value={summary}
                        onChange={(e) => setSummary(e.target.value)}
                        style={{
                            background: 'var(--ctp-surface0)',
                            color: 'var(--ctp-text)',
                            border: '1px solid var(--ctp-surface2)',
                        }}
                    />
                    <Textarea
                        header="Description (optional)"
                        placeholder="Add notes about this event..."
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        style={{
                            background: 'var(--ctp-surface0)',
                            color: 'var(--ctp-text)',
                            border: '1px solid var(--ctp-surface2)',
                            minHeight: '100px'
                        }}
                    />
                    <Input
                        header="Location (optional)"
                        placeholder="Conference Room A, Online, etc."
                        value={location}
                        onChange={(e) => setLocation(e.target.value)}
                        style={{
                            background: 'var(--ctp-surface0)',
                            color: 'var(--ctp-text)',
                            border: '1px solid var(--ctp-surface2)',
                        }}
                    />
                </Section>

                <Section header="Time">
                    <Cell
                        Component="label"
                        after={<Switch checked={isAllDay} onChange={(e) => setIsAllDay(e.target.checked)} />}
                    >
                        <Text style={{ color: 'var(--ctp-text)' }}>All Day Event</Text>
                    </Cell>

                    <Input
                        header="Start Time"
                        type="datetime-local"
                        value={start}
                        onChange={(e) => setStart(e.target.value)}
                        style={{
                            background: 'var(--ctp-surface0)',
                            color: 'var(--ctp-text)',
                            border: '1px solid var(--ctp-surface2)',
                        }}
                    />
                    <Input
                        header="End Time"
                        type="datetime-local"
                        value={end}
                        onChange={(e) => setEnd(e.target.value)}
                        style={{
                            background: 'var(--ctp-surface0)',
                            color: 'var(--ctp-text)',
                            border: '1px solid var(--ctp-surface2)',
                        }}
                    />
                </Section>

                <div style={{ height: 100 }} />
            </List>
        </div>
    );
}
