import { useState, useEffect } from 'react';
import { EventList } from './components/EventList';
import { CreateEvent } from './components/CreateEvent';
import { Plus } from 'lucide-react';
import { projectId, publicAnonKey } from './utils/supabase/info';

export interface Event {
  id: string;
  title: string;
  date: string;
  time?: string;
  description?: string;
  location?: string;
  duration?: number;
}

const API_URL = `https://${projectId}.supabase.co/functions/v1/make-server-7109387c`;

export default function App() {
  const [events, setEvents] = useState<Event[]>([]);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [editingEvent, setEditingEvent] = useState<Event | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Load events from server
  useEffect(() => {
    loadEvents();
  }, []);

  const loadEvents = async () => {
    try {
      const response = await fetch(`${API_URL}/events`, {
        headers: {
          'Authorization': `Bearer ${publicAnonKey}`,
        },
      });
      
      if (!response.ok) {
        throw new Error(`Failed to load events: ${response.statusText}`);
      }
      
      const data = await response.json();
      setEvents(data.events || []);
    } catch (error) {
      console.error('Error loading events:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCreateEvent = async (event: Omit<Event, 'id'>) => {
    try {
      const response = await fetch(`${API_URL}/events`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${publicAnonKey}`,
        },
        body: JSON.stringify(event),
      });

      if (!response.ok) {
        throw new Error(`Failed to create event: ${response.statusText}`);
      }

      const newEvent = await response.json();
      setEvents([...events, newEvent]);
      setIsCreateOpen(false);
    } catch (error) {
      console.error('Error creating event:', error);
    }
  };

  const handleDeleteEvent = async (id: string) => {
    try {
      const response = await fetch(`${API_URL}/events/${id}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${publicAnonKey}`,
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to delete event: ${response.statusText}`);
      }

      setEvents(events.filter(event => event.id !== id));
    } catch (error) {
      console.error('Error deleting event:', error);
    }
  };

  const handleUpdateEvent = async (id: string, updatedEvent: Omit<Event, 'id'>) => {
    try {
      const response = await fetch(`${API_URL}/events/${id}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${publicAnonKey}`,
        },
        body: JSON.stringify(updatedEvent),
      });

      if (!response.ok) {
        throw new Error(`Failed to update event: ${response.statusText}`);
      }

      const updated = await response.json();
      setEvents(events.map(event => event.id === id ? updated : event));
      setEditingEvent(null);
    } catch (error) {
      console.error('Error updating event:', error);
    }
  };

  const handleEditEvent = (event: Event) => {
    setEditingEvent(event);
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
          <EventList events={events} onDeleteEvent={handleDeleteEvent} onEditEvent={handleEditEvent} />
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
            onClose={() => setEditingEvent(null)}
            onCreate={(updatedEvent) => handleUpdateEvent(editingEvent.id, updatedEvent)}
            initialEvent={editingEvent}
          />
        )}
      </div>
    </div>
  );
}