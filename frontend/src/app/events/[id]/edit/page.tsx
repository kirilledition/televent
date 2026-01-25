'use client';

import { useQuery } from '@tanstack/react-query';
import { EventForm } from '@/components/EventForm';
import { api } from '@/lib/api';
import { useParams, useRouter } from 'next/navigation';

export default function EditEventPage() {
    const params = useParams();
    const id = params.id as string;
    const router = useRouter();

    const { data: event, isLoading, error } = useQuery({
        queryKey: ['events', id],
        queryFn: () => api.getEvent(id),
        enabled: !!id,
    });

    if (isLoading) return <div className="p-8 text-center">Loading...</div>;
    if (error) return (
        <div className="p-8 text-center">
            <p className="text-red mb-4">Error loading event</p>
            <button onClick={() => router.push('/')} className="btn-secondary">Go Back</button>
        </div>
    );

    return (
        <main className="min-h-screen bg-base text-text p-4 pb-20">
            <header className="mb-6">
                <h1 className="text-xl font-bold text-sapphire">Edit Event</h1>
            </header>

            {event && <EventForm initialData={event} isEditing />}
        </main>
    );
}
