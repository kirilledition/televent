import { Event } from '../App';
import { EventItem } from './EventItem';

interface EventListProps {
  events: Event[];
  onDeleteEvent: (id: string) => void;
  onEditEvent: (event: Event) => void;
}

export function EventList({ events, onDeleteEvent, onEditEvent }: EventListProps) {
  // Sort events by date and time
  const sortedEvents = [...events].sort((a, b) => {
    const dateA = new Date(`${a.date} ${a.time || '00:00'}`);
    const dateB = new Date(`${b.date} ${b.time || '00:00'}`);
    return dateA.getTime() - dateB.getTime();
  });

  // Group events by date
  const groupedEvents = sortedEvents.reduce((acc, event) => {
    const date = event.date;
    if (!acc[date]) {
      acc[date] = [];
    }
    acc[date].push(event);
    return acc;
  }, {} as Record<string, Event[]>);

  const formatDateHeader = (dateStr: string) => {
    const date = new Date(dateStr + 'T00:00:00');
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    const tomorrow = new Date(today);
    tomorrow.setDate(tomorrow.getDate() + 1);

    if (date.getTime() === today.getTime()) {
      return 'Today';
    } else if (date.getTime() === tomorrow.getTime()) {
      return 'Tomorrow';
    } else {
      return date.toLocaleDateString('en-US', { 
        weekday: 'long', 
        month: 'long', 
        day: 'numeric',
        year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined
      });
    }
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
      {Object.entries(groupedEvents).map(([date, dateEvents]) => (
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