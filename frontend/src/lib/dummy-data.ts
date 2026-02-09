import { Event } from '@/types/event'

export const DUMMY_EVENTS: Event[] = [
  {
    id: '1',
    title: 'Team Meeting',
    date: new Date().toISOString().split('T')[0], // Today
    time: '10:00',
    duration: 60,
    location: 'Conference Room A',
    description: 'Weekly team sync to discuss project status.',
  },
  {
    id: '2',
    title: 'Lunch with Client',
    date: new Date().toISOString().split('T')[0], // Today
    time: '12:30',
    duration: 90,
    location: 'Italian Restaurant',
    description: 'Discussing the new contract proposal.',
  },
  {
    id: '3',
    title: 'Code Review',
    date: new Date(Date.now() + 86400000).toISOString().split('T')[0], // Tomorrow
    time: '14:00',
    duration: 45,
    description: 'Reviewing the latest PRs for the frontend migration.',
  },
]
