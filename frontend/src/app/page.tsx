'use client';

import { useEffect, useMemo } from 'react';
import { useRouter } from 'next/navigation';
import {
  List,
  Section,
  Cell,
  Headline,
  Text,
  Spinner,
  Button,
} from '@telegram-apps/telegram-ui';
import { mainButton } from '@telegram-apps/sdk-react';
import useSWR from 'swr';
import { api, Event, User } from '@/lib/api';

export default function Home() {
  const router = useRouter();

  // 1. Fetch User
  const { data: user, error: userError, isLoading: userLoading } = useSWR<User, Error>(
    'user',
    api.getMe
  );

  // 2. Fetch Calendars
  const { data: calendars } = useSWR(
    user ? 'calendars' : null,
    api.getCalendars
  );

  const calendarId = calendars?.[0]?.id;

  // 3. Fetch Events
  const { data: events, error: eventsError, isLoading: eventsLoading } = useSWR(
    calendarId ? ['events', calendarId] : null,
    () => api.getEvents({ calendar_id: calendarId! })
  );

  // Configure Main Button
  useEffect(() => {
    if (!mainButton.isMounted()) return;

    const handleAddClick = () => {
      router.push('/create');
    };

    mainButton.setParams({
      text: 'ADD EVENT',
      isVisible: true,
      isEnabled: true,

    });

    // Cleanup click listeners
    const off = mainButton.onClick(handleAddClick);

    return () => {
      off();
      // Hide button when leaving (cleanup)
      mainButton.setParams({ isVisible: false });
    };
  }, [router]);

  // Ensure MainButton is mounted
  useEffect(() => {
    try {
      if (!mainButton.isMounted()) mainButton.mount();
    } catch (e) {
      console.error("Failed to mount mainButton", e);
    }
  }, []);

  // Group events by date
  const groupedEvents = useMemo(() => {
    if (!events) return {};

    return events.reduce((acc, event) => {
      const date = new Date(event.start).toLocaleDateString(undefined, {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
      });

      if (!acc[date]) acc[date] = [];
      acc[date].push(event);
      return acc;
    }, {} as Record<string, Event[]>);
  }, [events]);

  if (userLoading) {
    return <div style={{ display: 'flex', justifyContent: 'center', padding: 20 }}><Spinner size="l" /></div>;
  }

  if (userError) {
    return (
      <List>
        <Section>
          <Cell>
            <Text style={{ color: 'var(--tgui--destructive_text_color)' }}>
              Failed to load user: {userError.message}
            </Text>
          </Cell>
        </Section>
      </List>
    )
  }

  return (
    <List>
      <Section header="Your Schedule">
        {!calendarId && !eventsLoading && (
          <Cell><Text>No calendar found.</Text></Cell>
        )}

        {eventsLoading && <Cell><Spinner size="s" /></Cell>}

        {events && events.length === 0 && (
          <Cell multiline><Text>No upcoming events. Tap &quot;Add Event&quot; to start planning.</Text></Cell>
        )}
      </Section>

      {Object.entries(groupedEvents).map(([date, dayEvents]) => (
        <Section key={date} header={date}>
          {dayEvents.map((event) => (
            <Cell
              key={event.id}
              subhead={new Date(event.start).toLocaleTimeString(undefined, {
                hour: '2-digit',
                minute: '2-digit',
              })}
              description={event.location}
              before={
                <div style={{
                  width: 4,
                  height: 40,
                  borderRadius: 2,
                  background: event.status === 'Cancelled' ? 'var(--tgui--destructive_text_color)' : 'var(--tgui--link_color)',
                  marginRight: 12
                }} />
              }
            >
              <Headline weight="2" style={{
                textDecoration: event.status === 'Cancelled' ? 'line-through' : 'none'
              }}>
                {event.summary}
              </Headline>
            </Cell>
          ))}
        </Section>
      ))}

      <div style={{ height: 100 }} /> {/* Spacer for MainButton */}

      <Section header="Quick Actions">
        <div style={{ padding: '0 16px 16px', display: 'flex', gap: 8 }}>
          <Button size="l" stretched onClick={() => router.push('/create')}>
            Add Event
          </Button>
          <Button size="l" mode="bezeled" stretched onClick={() => router.push('/devices')}>
            Devices
          </Button>
        </div>
      </Section>

      <Section footer="Successfully initialized with Next.js + Typeshare">
      </Section>
    </List>
  );
}
