'use client';

import { useEffect, useMemo, useState } from 'react';
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
import { mainButton, hapticFeedback } from '@telegram-apps/sdk-react';
import useSWR, { mutate } from 'swr';
import { format, parseISO } from 'date-fns';
import { api, Event, User } from '@/lib/api';

export default function Home() {
  const router = useRouter();
  const [deleteConfirm, setDeleteConfirm] = useState<{ id: string; summary: string } | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  // 1. Fetch User
  const { data: user, error: userError, isLoading: userLoading } = useSWR<User, Error>(
    'user',
    api.getMe
  );

  // 2. Fetch Events (no calendar_id needed - user-scoped)
  const { data: events, error: eventsError, isLoading: eventsLoading, mutate: mutateEvents } = useSWR<Event[], Error>(
    user ? 'events' : null,
    () => api.getEvents()
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

  // Handle event deletion
  const handleDeleteClick = (event: Event) => {
    setDeleteConfirm({ id: event.id, summary: event.summary });
  };

  const confirmDelete = async () => {
    if (!deleteConfirm) return;

    setIsDeleting(true);
    try {
      // Optimistic update - remove from local state immediately
      const optimisticEvents = events?.filter(e => e.id !== deleteConfirm.id) || [];
      mutateEvents(optimisticEvents, false);

      // Perform actual deletion
      await api.deleteEvent(deleteConfirm.id);

      // Success feedback
      try {
        hapticFeedback.notificationOccurred('success');
      } catch {
        // Ignore if haptic feedback not available
      }

      // Revalidate to ensure consistency
      mutateEvents();

    } catch (error) {
      // Rollback on error
      mutateEvents();
      console.error('Failed to delete event:', error);

      try {
        hapticFeedback.notificationOccurred('error');
      } catch {
        // Ignore if haptic feedback not available
      }

      alert(`Failed to delete event: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setIsDeleting(false);
      setDeleteConfirm(null);
    }
  };

  const cancelDelete = () => {
    setDeleteConfirm(null);
  };

  // Group events by date
  const groupedEvents = useMemo(() => {
    if (!events) return {};

    // Sort events by start time
    const sortedEvents = [...events].sort((a, b) => {
      const dateA = a.start || a.start_date || '';
      const dateB = b.start || b.start_date || '';
      return dateA.localeCompare(dateB);
    });

    return sortedEvents.reduce((acc, event) => {
      try {
        const eventDate = event.start ? parseISO(event.start) : (event.start_date ? parseISO(event.start_date) : new Date());
        const dateKey = format(eventDate, 'EEE, MMM d');

        if (!acc[dateKey]) acc[dateKey] = [];
        acc[dateKey].push(event);
      } catch (error) {
        console.error('Error formatting event date:', error, event);
      }
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
            <Text style={{ color: 'var(--ctp-red)' }}>
              Failed to load user: {userError.message}
            </Text>
          </Cell>
        </Section>
      </List>
    );
  }

  if (eventsError) {
    return (
      <List>
        <Section>
          <Cell>
            <Text style={{ color: 'var(--ctp-red)' }}>
              Failed to load events: {eventsError.message}
            </Text>
          </Cell>
        </Section>
      </List>
    );
  }

  return (
    <>
      <List>
        <Section header="Your Schedule">
          {eventsLoading && (
            <Cell>
              <div style={{ display: 'flex', justifyContent: 'center', padding: '1rem' }}>
                <Spinner size="s" />
              </div>
            </Cell>
          )}

          {!eventsLoading && events && events.length === 0 && (
            <Cell multiline>
              <Text style={{ color: 'var(--ctp-subtext0)' }}>
                No upcoming events. Tap &quot;Add Event&quot; to start planning.
              </Text>
            </Cell>
          )}
        </Section>

        {!eventsLoading && Object.entries(groupedEvents).map(([date, dayEvents]) => (
          <Section key={date} header={date}>
            {dayEvents.map((event) => {
              const eventTime = event.start ? parseISO(event.start) : null;
              const timeDisplay = event.is_all_day
                ? 'All Day'
                : eventTime
                  ? format(eventTime, 'h:mm a')
                  : '';

              return (
                <Cell
                  key={event.id}
                  subhead={timeDisplay}
                  description={event.description || event.location}
                  before={
                    <div style={{
                      width: 4,
                      height: 40,
                      borderRadius: 2,
                      background: event.status === 'Cancelled'
                        ? 'var(--ctp-red)'
                        : 'var(--ctp-sapphire)',
                      marginRight: 12
                    }} />
                  }
                  after={
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteClick(event);
                      }}
                      style={{
                        background: 'var(--ctp-red)',
                        color: 'var(--ctp-base)',
                        border: 'none',
                        borderRadius: '0.375rem',
                        padding: '0.5rem 0.75rem',
                        cursor: 'pointer',
                        fontSize: '0.875rem',
                        fontWeight: 600,
                        transition: 'all 0.2s ease',
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = 'var(--ctp-maroon)';
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = 'var(--ctp-red)';
                      }}
                    >
                      Delete
                    </button>
                  }
                >
                  <Headline weight="2" style={{
                    textDecoration: event.status === 'Cancelled' ? 'line-through' : 'none',
                    color: 'var(--ctp-text)'
                  }}>
                    {event.summary}
                  </Headline>
                </Cell>
              );
            })}
          </Section>
        ))}

        <div style={{ height: 100 }} />

        <Section header="Quick Actions">
          <div style={{ padding: '0 16px 16px', display: 'flex', gap: 8 }}>
            <Button
              size="l"
              stretched
              onClick={() => router.push('/create')}
              style={{
                background: 'var(--ctp-sapphire)',
                color: 'var(--ctp-base)'
              }}
            >
              Add Event
            </Button>
            <Button
              size="l"
              mode="bezeled"
              stretched
              onClick={() => router.push('/devices')}
            >
              Devices
            </Button>
          </div>
        </Section>
      </List>

      {/* Delete Confirmation Modal */}
      {deleteConfirm && (
        <div className="modal-backdrop" onClick={cancelDelete}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2 style={{
              fontSize: '1.5rem',
              fontWeight: 700,
              marginBottom: '1rem',
              color: 'var(--ctp-text)'
            }}>
              Delete Event?
            </h2>
            <p style={{
              marginBottom: '1.5rem',
              color: 'var(--ctp-subtext0)'
            }}>
              Are you sure you want to delete &quot;{deleteConfirm.summary}&quot;? This action cannot be undone.
            </p>
            <div style={{ display: 'flex', gap: '0.75rem', justifyContent: 'flex-end' }}>
              <button
                onClick={cancelDelete}
                disabled={isDeleting}
                className="btn-secondary"
              >
                Cancel
              </button>
              <button
                onClick={confirmDelete}
                disabled={isDeleting}
                className="btn-destructive"
              >
                {isDeleting ? 'Deleting...' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
