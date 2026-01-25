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
import useSWR from 'swr';
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

    // Fetch prerequisites
    const { data: user } = useSWR<User, Error>('user', api.getMe);
    const { data: calendars } = useSWR(user ? 'calendars' : null, api.getCalendars);
    const calendarId = calendars?.[0]?.id;

    // Handle Save
    const handleSave = useCallback(async () => {
        if (!calendarId || !user) return;
        if (!summary) {
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError("Summary is required");
            return;
        }

        setIsSubmitting(true);
        setError(null);

        try {
            await api.createEvent({
                calendar_id: calendarId,
                uid: crypto.randomUUID(),
                summary,
                description: description || undefined,
                location: location || undefined,
                start: new Date(start).toISOString(),
                end: new Date(end).toISOString(),
                is_all_day: isAllDay,
                timezone: user.timezone || 'UTC',
                rrule: undefined,
            });

            try { hapticFeedback.notificationOccurred('success'); } catch { }
            router.back();
        } catch (err) {
            console.error(err);
            try { hapticFeedback.notificationOccurred('error'); } catch { }
            setError(err instanceof Error ? err.message : 'Failed to create event');
            setIsSubmitting(false);
        }
    }, [calendarId, user, summary, description, location, start, end, isAllDay, router]);

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

    if (!calendarId) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', padding: 20 }}>
                <Spinner size="l" />
            </div>
        );
    }

    return (
        <List>
            {error && (
                <Section>
                    <Cell><Text style={{ color: 'var(--tgui--destructive_text_color)' }}>{error}</Text></Cell>
                </Section>
            )}

            <Section header="Event Details">
                <Input
                    header="Summary"
                    placeholder="New Event"
                    value={summary}
                    onChange={(e) => setSummary(e.target.value)}
                />
                <Textarea
                    header="Description"
                    placeholder="Notes"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                />
                <Input
                    header="Location"
                    placeholder="Add Location"
                    value={location}
                    onChange={(e) => setLocation(e.target.value)}
                />
            </Section>

            <Section header="Time">
                <Cell
                    Component="label"
                    after={<Switch checked={isAllDay} onChange={(e) => setIsAllDay(e.target.checked)} />}
                >
                    All Day
                </Cell>

                <Input
                    header="Starts"
                    type="datetime-local"
                    value={start}
                    onChange={(e) => setStart(e.target.value)}
                />
                <Input
                    header="Ends"
                    type="datetime-local"
                    value={end}
                    onChange={(e) => setEnd(e.target.value)}
                />
            </Section>
        </List>
    );
}
