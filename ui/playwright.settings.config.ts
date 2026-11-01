import { defineConfig } from '@playwright/test';

const port = Number(process.env.TMC_SETTINGS_E2E_PORT ?? 5178);
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: './tests/e2e',
  testMatch: 'settings-scroll.spec.ts',
  timeout: 30_000,
  workers: 1,
  use: { baseURL, channel: process.platform === 'win32' ? 'msedge' : undefined },
  projects: [
    { name: 'settings-1366', use: { viewport: { width: 1366, height: 768 } } },
    { name: 'settings-1440', use: { viewport: { width: 1440, height: 900 } } },
    { name: 'settings-1920', use: { viewport: { width: 1920, height: 1080 } } },
  ],
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    url: baseURL,
    reuseExistingServer: false,
  },
});
