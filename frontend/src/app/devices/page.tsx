'use client';

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import {
    List,
    Section,
    Cell,
    Button,
    Input,
    Text,
    Spinner,
    Modal,
} from '@telegram-apps/telegram-ui';
import { backButton, hapticFeedback } from '@telegram-apps/sdk-react';
import useSWR, { mutate } from 'swr';
import { api, User, DeviceListItem, DevicePasswordResponse } from '@/lib/api';

export default function DevicesPage() {
    const router = useRouter();

    const [isCreating, setIsCreating] = useState(false);
    const [newDeviceName, setNewDeviceName] = useState('');
    const [createdDevice, setCreatedDevice] = useState<DevicePasswordResponse | null>(null);
    const [isPasswordVisible, setIsPasswordVisible] = useState(false);

    // Fetch User & Devices
    const { data: user } = useSWR<User, Error>('user', api.getMe);
    const { data: devices, isLoading } = useSWR<DeviceListItem[]>(
        user ? ['devices', user.id] : null,
        () => api.getDevices(user!.id)
    );

    // Setup SDK
    useEffect(() => {
        if (!backButton.isMounted()) backButton.mount();
        try {
            // @ts-expect-error hapticFeedback.mount might not exist in types but is needed in some versions
            if (hapticFeedback.mount && !hapticFeedback.isMounted()) hapticFeedback.mount();
        } catch { }

        backButton.show();
        const onClick = () => router.back();
        const cleanup = backButton.onClick(onClick);
        return () => {
            cleanup();
            backButton.hide();
        };
    }, [router]);

    const handleCreate = async () => {
        if (!user || !newDeviceName) return;

        try {
            const device = await api.createDevice(user.id, newDeviceName);
            setCreatedDevice(device);
            setNewDeviceName('');
            setIsCreating(false);
            try { hapticFeedback.notificationOccurred('success'); } catch { }
            // Refresh list
            mutate(['devices', user.id]);
        } catch (err) {
            console.error(err);
            try { hapticFeedback.notificationOccurred('error'); } catch { }
        }
    };

    const togglePassword = () => {
        setIsPasswordVisible(!isPasswordVisible);
        try { hapticFeedback.impactOccurred('light'); } catch { }
    };

    const handleDelete = async (deviceId: string) => {
        if (!user) return;
        if (!confirm('Revoke this device password?')) return;

        try {
            await api.deleteDevice(user.id, deviceId);
            mutate(['devices', user.id]);
            try { hapticFeedback.notificationOccurred('success'); } catch { }
        } catch (err) {
            console.error(err);
            try { hapticFeedback.notificationOccurred('error'); } catch { }
        }
    };

    return (
        <List>
            <Section header="Connected Devices" footer="Use these passwords to log in via CalDAV on your phone or computer.">
                {isLoading && <Cell><Spinner size="s" /></Cell>}

                {devices?.map((device) => (
                    <Cell
                        key={device.id}
                        subhead={new Date(device.created_at).toLocaleDateString()}
                        after={
                            <Button
                                mode="plain"
                                size="s"
                                onClick={(e) => {
                                    e.preventDefault();
                                    handleDelete(device.id);
                                }}
                                style={{ color: 'var(--tgui--destructive_text_color)' }}
                            >
                                Revoke
                            </Button>
                        }
                    >
                        {device.name}
                    </Cell>
                ))}

                {devices && devices.length === 0 && (
                    <Cell><Text>No devices connected yet.</Text></Cell>
                )}

                <Cell>
                    <Button size="m" stretched onClick={() => setIsCreating(true)}>
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
                    <Cell
                        onClick={togglePassword}
                        description="Tap to reveal"
                    >
                        <div style={{
                            fontSize: 18,
                            fontWeight: 600,
                            fontFamily: 'monospace',
                            cursor: 'pointer',
                            filter: isPasswordVisible ? 'none' : 'blur(4px)',
                            transition: 'filter 0.2s',
                        }}>
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
                                <Button stretched onClick={handleCreate} disabled={!newDeviceName}>
                                    Generate Password
                                </Button>
                            </Cell>
                        </Section>
                    </List>
                </Modal>
            )}
        </List>
    );
}
