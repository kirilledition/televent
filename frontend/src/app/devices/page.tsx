'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import {
  List,
  Section,
  Cell,
  Button,
  Input,
  Text,
  Spinner,
  Modal,
} from '@telegram-apps/telegram-ui'
import { backButton, hapticFeedback } from '@tma.js/sdk-react'
import useSWR, { mutate } from 'swr'
import { api, User, DeviceListItem, DevicePasswordResponse } from '@/lib/api'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'

export default function DevicesPage() {
  const router = useRouter()

  const [isCreating, setIsCreating] = useState(false)
  const [newDeviceName, setNewDeviceName] = useState('')
  const [createdDevice, setCreatedDevice] =
    useState<DevicePasswordResponse | null>(null)
  const [isPasswordVisible, setIsPasswordVisible] = useState(false)

  // Fetch User & Devices (no userId needed - uses authenticated user)
  const { data: user } = useSWR<User, Error>('user', () => api.getMe())
  const { data: devices, isLoading } = useSWR<DeviceListItem[]>(
    user ? 'devices' : null,
    () => api.getDevices()
  )

  // Setup SDK
  useEffect(() => {
    if (!backButton) return

    // Mount if necessary (safely checking availability)
    if (
      backButton.mount &&
      backButton.mount.isAvailable() &&
      !backButton.isMounted()
    ) {
      backButton.mount()
    }

    try {
      if (
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (hapticFeedback as any).mount &&
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        !(hapticFeedback as any).isMounted()
      ) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        ;(hapticFeedback as any).mount()
      }
    } catch {}

    if (backButton.isMounted()) {
      backButton.show()
      const cleanup = backButton.onClick(() => router.back())

      return () => {
        cleanup()
        backButton.hide()
      }
    }
  }, [router])

  const handleCreate = async () => {
    if (!user || !newDeviceName) return

    try {
      const device = await api.createDevice(newDeviceName)
      setCreatedDevice(device)
      setNewDeviceName('')
      setIsCreating(false)
      try {
        hapticFeedback?.notificationOccurred('success')
      } catch {}
      // Refresh list
      mutate('devices')
    } catch (err) {
      console.error(err)
      try {
        hapticFeedback?.notificationOccurred('error')
      } catch {}
    }
  }

  const togglePassword = () => {
    setIsPasswordVisible(!isPasswordVisible)
    try {
      hapticFeedback?.impactOccurred('light')
    } catch {}
  }

  const handleDelete = async (deviceId: string) => {
    if (!user) return

    try {
      await api.deleteDevice(deviceId)
      mutate('devices')
      try {
        hapticFeedback?.notificationOccurred('success')
      } catch {}
    } catch (err) {
      console.error(err)
      try {
        hapticFeedback?.notificationOccurred('error')
      } catch {}
    }
  }

  return (
    <div style={{ background: 'var(--ctp-base)', minHeight: '100vh' }}>
      <List>
        <Section
          header="Connected Devices"
          footer="Use these passwords to log in via CalDAV on your phone or computer."
        >
          {isLoading && (
            <Cell>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'center',
                  padding: '1rem',
                }}
              >
                <Spinner size="s" />
              </div>
            </Cell>
          )}

          {devices?.map((device) => (
            <Cell
              key={device.id}
              subhead={new Date(device.created_at).toLocaleDateString()}
              after={
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button
                      mode="plain"
                      size="s"
                      aria-label={`Revoke device: ${device.name}`}
                      style={{
                        color: 'var(--ctp-red)',
                        fontWeight: 600,
                      }}
                    >
                      Revoke
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent
                    className="border-none shadow-xl"
                    style={{
                      backgroundColor: 'var(--ctp-base)',
                      color: 'var(--ctp-text)',
                    }}
                  >
                    <AlertDialogHeader>
                      <AlertDialogTitle style={{ color: 'var(--ctp-text)' }}>
                        Revoke Device
                      </AlertDialogTitle>
                      <AlertDialogDescription
                        style={{ color: 'var(--ctp-subtext0)' }}
                      >
                        Are you sure you want to revoke &quot;{device.name}
                        &quot;? This device will no longer be able to sync using
                        its CalDAV password.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel
                        style={{
                          backgroundColor: 'var(--ctp-surface0)',
                          color: 'var(--ctp-text)',
                          borderColor: 'transparent',
                        }}
                      >
                        Cancel
                      </AlertDialogCancel>
                      <AlertDialogAction
                        onClick={() => handleDelete(device.id)}
                        style={{
                          backgroundColor: 'var(--ctp-red)',
                          color: 'var(--ctp-base)',
                        }}
                      >
                        Revoke
                      </AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              }
            >
              <Text style={{ color: 'var(--ctp-text)' }}>{device.name}</Text>
            </Cell>
          ))}

          {devices && devices.length === 0 && (
            <Cell>
              <Text style={{ color: 'var(--ctp-subtext0)' }}>
                No devices connected yet.
              </Text>
            </Cell>
          )}

          <Cell>
            <Button
              size="m"
              stretched
              onClick={() => setIsCreating(true)}
              style={{
                background: 'var(--ctp-sapphire)',
                color: 'var(--ctp-base)',
              }}
            >
              Create New Device Password
            </Button>
          </Cell>
        </Section>

        {createdDevice && (
          <Section header="New Password Created">
            <Cell multiline>
              <Text>
                Use this password to log in. It will only be shown once.
              </Text>
            </Cell>
            <Cell onClick={togglePassword} description="Tap to reveal">
              <div
                style={{
                  fontSize: 18,
                  fontWeight: 600,
                  fontFamily: 'monospace',
                  cursor: 'pointer',
                  filter: isPasswordVisible ? 'none' : 'blur(4px)',
                  transition: 'filter 0.2s',
                }}
              >
                {createdDevice.password}
              </div>
            </Cell>
            <Cell>
              <Button
                mode="bezeled"
                stretched
                onClick={() => setCreatedDevice(null)}
              >
                Done
              </Button>
            </Cell>
          </Section>
        )}

        {isCreating && (
          <Modal
            header={<Modal.Header>New Device</Modal.Header>}
            open={isCreating}
            onOpenChange={setIsCreating}
          >
            <List>
              <Section>
                <Input
                  header="Device Name"
                  placeholder="e.g. My iPhone"
                  value={newDeviceName}
                  onChange={(e) => setNewDeviceName(e.target.value)}
                />
                <Cell>
                  <Button
                    stretched
                    onClick={handleCreate}
                    disabled={!newDeviceName}
                  >
                    Generate Password
                  </Button>
                </Cell>
              </Section>
            </List>
          </Modal>
        )}
      </List>
    </div>
  )
}
