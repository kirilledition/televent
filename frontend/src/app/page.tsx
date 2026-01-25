'use client'

import { EventList } from '@/components/EventList'
import Link from 'next/link'

export default function Home() {
  return (
    <main className="bg-base text-text min-h-screen pb-20">
      <header className="bg-mantle/80 border-surface0 sticky top-0 z-10 flex items-center justify-between border-b px-4 py-3 backdrop-blur-md">
        <h1 className="from-sapphire to-blue bg-gradient-to-r bg-clip-text text-xl font-bold text-transparent">
          My Calendar
        </h1>
      </header>

      <EventList />

      <Link
        href="/create"
        className="bg-sapphire shadow-sapphire/30 fixed right-6 bottom-6 z-50 flex h-14 w-14 items-center justify-center rounded-full text-3xl text-base shadow-lg transition-all hover:scale-110 active:scale-95"
      >
        +
      </Link>
    </main>
  )
}
