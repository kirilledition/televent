import { Event } from '../App';
import { Trash2, MapPin, Clock } from 'lucide-react';

interface EventItemProps {
  event: Event;
  onDelete: (id: string) => void;
  onEdit: (event: Event) => void;
}

export function EventItem({ event, onDelete, onEdit }: EventItemProps) {
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
        className="flex-1 min-w-0 pl-2 cursor-pointer"
        onClick={() => onEdit(event)}
      >
        <div className="flex items-start justify-between gap-3 mb-2">
          <h3 className="text-lg font-medium" style={{ color: 'var(--ctp-text)' }}>{event.title}</h3>
          
          {/* Delete button - always visible on mobile */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete(event.id);
            }}
            className="p-2 rounded-lg transition-all hover:opacity-70"
            style={{ backgroundColor: 'var(--ctp-surface0)' }}
            aria-label="Delete event"
          >
            <Trash2 className="w-4 h-4" style={{ color: 'var(--ctp-subtext0)' }} />
          </button>
        </div>
        
        <div className="space-y-1">
          {event.time && (
            <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--ctp-subtext1)' }}>
              <Clock className="w-4 h-4" />
              <span>{event.time}</span>
              {event.duration && (
                <span style={{ color: 'var(--ctp-overlay1)' }}>
                  • {formatDuration(event.duration)}
                </span>
              )}
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
