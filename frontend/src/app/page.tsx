'use client';

import { EventList } from '@/components/EventList';
import Link from 'next/link';

export default function Home() {
  return (
    <main className="min-h-screen bg-base text-text pb-20">
      <header className="sticky top-0 z-10 bg-mantle/80 backdrop-blur-md border-b border-surface0 px-4 py-3 flex items-center justify-between">
        <h1 className="text-xl font-bold bg-gradient-to-r from-sapphire to-blue text-transparent bg-clip-text">
          My Calendar
        </h1>
      </header>

      <EventList />

      <Link
        href="/create"
        className="fixed bottom-6 right-6 h-14 w-14 rounded-full bg-sapphire text-base shadow-lg shadow-sapphire/30 flex items-center justify-center text-3xl hover:scale-110 active:scale-95 transition-all z-50 text-base"
      >
        +
      </Link>
    </main>
  );
}
