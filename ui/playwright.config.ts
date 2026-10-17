import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:47320',
    channel: process.platform === 'win32' ? 'msedge' : undefined,
  },
  webServer: {
    command: 'node scripts/e2e-server.mjs',
    url: 'http://127.0.0.1:47320/api/health',
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
