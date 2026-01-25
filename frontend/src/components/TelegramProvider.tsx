'use client';

import { PropsWithChildren, useEffect, useState, Component, ErrorInfo, ReactNode } from 'react';
import {
    useSignal,
    miniApp,
    themeParams,
    viewport,
    useLaunchParams,
} from '@tma.js/sdk-react';
import { AppRoot } from '@telegram-apps/telegram-ui';
// import eruda from 'eruda'; // Removed for SSR safety

/**
 * Mocking the Telegram environment for local development.
 */
function useTelegramMock() {
    useEffect(() => {
        if (process.env.NODE_ENV === 'development' && typeof window !== 'undefined') {
            // Very basic check for being outside of Telegram
            const isTelegram = window.self !== window.top || window.location.search.includes('tgWebAppStartParam');

            if (!isTelegram) {
                console.log('🛠️ Mocking Telegram environment...');
                import('eruda').then((lib) => {
                    const eruda = lib.default;
                    try {
                        eruda.init();
                    } catch (e) {
                        console.error('Failed to init eruda', e);
                    }
                });
            }
        }
    }, []);
}

interface ErrorBoundaryProps {
    children: ReactNode;
    fallback?: ReactNode;
}

class ErrorBoundary extends Component<ErrorBoundaryProps, { hasError: boolean }> {
    constructor(props: ErrorBoundaryProps) {
        super(props);
        this.state = { hasError: false };
    }

    static getDerivedStateFromError(_: Error) {
        return { hasError: true };
    }

    componentDidCatch(error: Error, errorInfo: ErrorInfo) {
        console.error("Uncaught error:", error, errorInfo);
    }

    render() {
        if (this.state.hasError) {
            return this.props.fallback || <h1>Something went wrong.</h1>;
        }

        return this.props.children;
    }
}

function SDKBinder({ children }: { children: ReactNode }) {
    // Get signal values
    const viewportInstance = useSignal(viewport.state);


    // Initialize and mount components
    useEffect(() => {
        if (miniApp.mount.isAvailable() && !miniApp.isMounted()) {
            miniApp.mount();
        }
        if (themeParams.mount.isAvailable() && !themeParams.isMounted()) {
            themeParams.mount();
        }
        if (viewport.mount.isAvailable() && !viewport.isMounted()) {
            viewport.mount();
        }
    }, []);

    // Bind CSS variables
    useEffect(() => {
        return miniApp.bindCssVars();
    }, []);

    useEffect(() => {
        return themeParams.bindCssVars();
    }, []);

    useEffect(() => {
        if (viewportInstance) {
            return viewport.bindCssVars();
        }
    }, [viewportInstance]);

    return <>{children}</>;
}

function AppInitializer({ children }: PropsWithChildren) {
    try {
        // eslint-disable-next-line react-hooks/rules-of-hooks
        useLaunchParams();
    } catch (e) {
        // Fallback for non-Telegram environment
        return <>{children}</>;
    }

    return <SDKBinder>{children}</SDKBinder>;
}

export function TelegramProvider({ children }: PropsWithChildren) {
    useTelegramMock();

    const [isClient, setIsClient] = useState(false);

    useEffect(() => {
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setIsClient(true);
    }, []);

    if (!isClient) {
        return <div style={{ background: '#000', height: '100vh' }} />;
    }

    return (
        <AppRoot>
            <ErrorBoundary fallback={<>{children}</>}>
                <AppInitializer>{children}</AppInitializer>
            </ErrorBoundary>
        </AppRoot>
    );
}
