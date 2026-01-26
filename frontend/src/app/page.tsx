'use client'

import { EventList } from '@/components/EventList'
import { useMainButton } from '@/components/TelegramProvider'
import { useRouter } from 'next/navigation'
import { useEffect } from 'react'

export default function Home() {
  const router = useRouter()
  const mainButton = useMainButton()

  useEffect(() => {
    const handleClick = () => {
      router.push('/create')
    }

    if (mainButton) {
      mainButton.setText('ADD EVENT')
      mainButton.enable()
      mainButton.show()
      const cleanup = mainButton.onClick(handleClick)

      return () => {
        cleanup()
        mainButton.hide()
      }
    }
  }, [mainButton, router])

  return (
    <main className="bg-base text-text min-h-screen pb-20">
      <header className="bg-mantle/80 border-surface0 sticky top-0 z-10 flex items-center justify-between border-b px-4 py-3 backdrop-blur-md">
        <h1 className="from-sapphire to-blue bg-gradient-to-r bg-clip-text text-xl font-bold text-transparent">
          My Calendar
        </h1>
      </header>

      <EventList />
    </main>
  )
}
