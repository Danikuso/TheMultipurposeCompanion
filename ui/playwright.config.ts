import { defineConfig } from '@playwright/test';

const port = Number(process.env.TMC_E2E_PORT ?? 47320);

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    channel: process.platform === 'win32' ? 'msedge' : undefined,
  },
  webServer: {
    command: 'node scripts/e2e-server.mjs',
    url: `http://127.0.0.1:${port}/api/health`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
