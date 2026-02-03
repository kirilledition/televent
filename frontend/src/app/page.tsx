'use client';

import { useState, useEffect, useCallback } from 'react';
import { EventResponse, CreateEventRequest, UpdateEventRequest } from '@/types/schema';
import { api } from '@/lib/api';
// Updated imports to point to where they are actually located (in ui/_components for now)
import { EventList } from './ui/_components/EventList';
import { CreateEvent } from './ui/_components/CreateEvent';
import { Plus } from 'lucide-react';

export default function CalendarPage() {
  const [events, setEvents] = useState<EventResponse[]>([]);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingEvent, setEditingEvent] = useState<EventResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Load events
  useEffect(() => {
    loadEvents();
  }, []);

  const loadEvents = async () => {
    try {
      setIsLoading(true);
      const data = await api.getEvents();
      // Ensure we have an array (handle 204/empty)
      setEvents(Array.isArray(data) ? data : []);
    } catch (error) {
      console.error('Error loading events:', error);
      // Fallback empty array on error
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateEvent = async (request: CreateEventRequest) => {
    try {
      const newEvent = await api.createEvent(request);
      setEvents(prev => [...prev, newEvent]);
      setIsCreateOpen(false);
    } catch (error) {
      console.error('Error creating event:', error);
    }
  };

  // Memoized to keep prop stable for EventList -> EventItem
  const handleDeleteEvent = useCallback(async (id: string) => {
    try {
      await api.deleteEvent(id);
      setEvents(prev => prev.filter(e => e.id !== id));
    } catch (error) {
      console.error('Error deleting event:', error);
    }
  }, []);

  const handleUpdateEvent = async (request: CreateEventRequest) => {
    if (!editingEvent) return;

    try {
      // Map CreateEventRequest to UpdateEventRequest (filtering out non-updateable fields like uid/timezone if not supported)
      const updateData: UpdateEventRequest = {
        summary: request.summary,
        description: request.description,
        location: request.location,
        start: request.start,
        end: request.end,
        is_all_day: request.is_all_day,
        rrule: request.rrule,
      };

      const updated = await api.updateEvent(editingEvent.id, updateData);
      setEvents(prev => prev.map(e => e.id === editingEvent.id ? updated : e));
      setEditingEvent(null);
    } catch (error) {
      console.error('Error updating event:', error);
    }
  };

  return (
    <div className="min-h-screen" style={{ backgroundColor: 'var(--ctp-base)' }}>
      <div className="max-w-2xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-semibold mb-1" style={{ color: 'var(--ctp-text)' }}>Calendar</h1>
          <p className="text-sm" style={{ color: 'var(--ctp-subtext0)' }}>Keep track of your events</p>
        </div>

        {/* New Event Button */}
        <button
          onClick={() => setIsCreateOpen(true)}
          className="flex items-center gap-2 px-5 py-3 mb-6 font-medium rounded-lg shadow-sm hover:opacity-90 transition-opacity w-full justify-center"
          style={{ backgroundColor: 'var(--ctp-mauve)', color: 'var(--ctp-crust)' }}
        >
          <Plus className="w-5 h-5" />
          <span>New event</span>
        </button>

        {/* Event List */}
        {isLoading ? (
          <div className="text-center py-16" style={{ color: 'var(--ctp-overlay0)' }}>
            <p>Loading events...</p>
          </div>
        ) : (
          <EventList events={events} onDeleteEvent={handleDeleteEvent} onEditEvent={setEditingEvent} />
        )}

        {/* Create Event Modal */}
        {isCreateOpen && (
          <CreateEvent
            onClose={() => setIsCreateOpen(false)}
            onCreate={handleCreateEvent}
          />
        )}

        {/* Edit Event Modal */}
        {editingEvent && (
          <CreateEvent
            initialEvent={editingEvent}
            onClose={() => setEditingEvent(null)}
            onCreate={handleUpdateEvent}
          />
        )}
      </div>
    </div>
  );
}
