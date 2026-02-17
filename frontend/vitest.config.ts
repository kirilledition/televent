import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import tsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
  plugins: [react(), tsconfigPaths()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./vitest.setup.ts'],
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/types/**/*',
        'src/**/*.d.ts',
        'src/**/layout.tsx',
        'src/**/page.tsx',
        'src/lib/dummy-data.ts',
        'src/components/ui/**/*', // Exclude shadcn ui components
        'src/components/TelegramProvider.tsx', // Telegram SDK integration - requires Telegram Mini App environment
        'src/components/QueryProvider.tsx', // Exclude provider
      ],
    },
  },
})
