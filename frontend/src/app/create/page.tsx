'use client';

import { EventForm } from '@/components/EventForm';

export default function CreateEventPage() {
    return (
        <main className="min-h-screen bg-base text-text p-4 pb-20">
            <header className="mb-6">
                <h1 className="text-xl font-bold text-sapphire">New Event</h1>
            </header>

            <EventForm />
        </main>
    );
}
