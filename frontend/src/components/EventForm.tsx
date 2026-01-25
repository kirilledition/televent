'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { api, CreateEventRequest, UpdateEventRequest, EventResponse } from '@/lib/api';

interface EventFormProps {
    initialData?: EventResponse;
    isEditing?: boolean;
}

export function EventForm({ initialData, isEditing = false }: EventFormProps) {
    const router = useRouter();
    const queryClient = useQueryClient();
    const [error, setError] = useState<string | null>(null);

    const [formData, setFormData] = useState({
        summary: initialData?.summary || '',
        description: initialData?.description || '',
        location: initialData?.location || '',
        start: initialData?.start ? new Date(initialData.start).toISOString().slice(0, 16) : new Date().toISOString().slice(0, 16),
        end: initialData?.end ? new Date(initialData.end).toISOString().slice(0, 16) : new Date(new Date().getTime() + 60 * 60000).toISOString().slice(0, 16),
        is_all_day: initialData?.is_all_day || false,
        timezone: (initialData?.timezone as string) || 'UTC',
    });

    const createMutation = useMutation({
        mutationFn: (data: CreateEventRequest) => api.createEvent(data),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['events'] });
            router.push('/');
            router.refresh();
        },
        onError: (err: Error) => setError(err.message),
    });

    const updateMutation = useMutation({
        mutationFn: (data: UpdateEventRequest) => api.updateEvent(initialData!.id, data),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['events'] });
            queryClient.invalidateQueries({ queryKey: ['events', initialData!.id] });
            router.push('/');
            router.refresh();
        },
        onError: (err: Error) => setError(err.message),
    });

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (!formData.summary) {
            setError('Summary is required');
            return;
        }

        const payload = {
            ...formData,
            start: new Date(formData.start).toISOString(),
            end: new Date(formData.end).toISOString(),
        };

        if (isEditing && initialData) {
            updateMutation.mutate(payload);
        } else {
            createMutation.mutate({
                ...payload,
                uid: crypto.randomUUID(), // Generate UID on frontend for new events
            });
        }
    };

    return (
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            {error && (
                <div className="bg-red/20 text-red border-red rounded-lg border p-3 text-sm">
                    {error}
                </div>
            )}

            <div>
                <label className="text-subtext0 mb-1 block text-sm font-medium">Title</label>
                <input
                    type="text"
                    value={formData.summary}
                    onChange={(e) => setFormData({ ...formData, summary: e.target.value })}
                    placeholder="Event title"
                    required
                />
            </div>

            <div>
                <label className="text-subtext0 mb-1 block text-sm font-medium">Description</label>
                <textarea
                    value={formData.description}
                    onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                    placeholder="Details..."
                    rows={3}
                />
            </div>

            <div>
                <label className="text-subtext0 mb-1 block text-sm font-medium">Location</label>
                <input
                    type="text"
                    value={formData.location}
                    onChange={(e) => setFormData({ ...formData, location: e.target.value })}
                    placeholder="Where?"
                />
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div>
                    <label className="text-subtext0 mb-1 block text-sm font-medium">Start</label>
                    <input
                        type="datetime-local"
                        value={formData.start}
                        onChange={(e) => setFormData({ ...formData, start: e.target.value })}
                        required
                    />
                </div>

                <div>
                    <label className="text-subtext0 mb-1 block text-sm font-medium">End</label>
                    <input
                        type="datetime-local"
                        value={formData.end}
                        onChange={(e) => setFormData({ ...formData, end: e.target.value })}
                        required
                    />
                </div>
            </div>

            <div className="flex items-center gap-2">
                <input
                    type="checkbox"
                    id="all_day"
                    checked={formData.is_all_day}
                    onChange={(e) => setFormData({ ...formData, is_all_day: e.target.checked })}
                    className="h-4 w-4 rounded border-gray-300 text-sapphire focus:ring-sapphire"
                />
                <label htmlFor="all_day" className="text-sm font-medium text-text">All Day</label>
            </div>

            <div className="mt-4 flex gap-3">
                <button
                    type="button"
                    onClick={() => router.back()}
                    className="btn-secondary flex-1"
                >
                    Cancel
                </button>
                <button
                    type="submit"
                    className="btn-primary flex-1"
                    disabled={createMutation.isPending || updateMutation.isPending}
                >
                    {createMutation.isPending || updateMutation.isPending ? 'Saving...' : (isEditing ? 'Update Event' : 'Create Event')}
                </button>
            </div>
        </form>
    );
}
