'use client';

import { PropsWithChildren, useEffect, useState, useMemo } from 'react';
import {
    useSignal,
    miniApp,
    themeParams,
    viewport,
    bindMiniAppCssVars,
    bindThemeParamsCssVars,
    bindViewportCssVars,
    useLaunchParams,
} from '@telegram-apps/sdk-react';
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

function AppInitializer({ children }: PropsWithChildren) {
    const lp = useLaunchParams();
    const isMobile = lp.platform === 'android' || lp.platform === 'ios';

    // Get signal values
    const miniAppInstance = useSignal(miniApp.state);
    const themeParamsInstance = useSignal(themeParams.state);
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
        return bindMiniAppCssVars();
    }, [miniAppInstance]);

    useEffect(() => {
        return bindThemeParamsCssVars();
    }, [themeParamsInstance]);

    useEffect(() => {
        if (viewportInstance) {
            return bindViewportCssVars();
        }
    }, [viewportInstance]);

    return <>{children}</>;
}

export function TelegramProvider({ children }: PropsWithChildren) {
    useTelegramMock();

    const [isClient, setIsClient] = useState(false);

    useEffect(() => {
        setIsClient(true);
    }, []);

    if (!isClient) {
        return <div style={{ background: '#000', height: '100vh' }} />;
    }

    return (
        <AppRoot>
            <AppInitializer>{children}</AppInitializer>
        </AppRoot>
    );
}
