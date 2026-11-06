import { defineConfig } from '@playwright/test';

const port = Number(process.env.TMC_GUIDE_QA_PORT ?? 47690);
export default defineConfig({
  testDir: './tests/e2e',
  testMatch: ['engineer-guide.spec.ts', 'layout.spec.ts'],
  workers: 1,
  timeout: 45_000,
  expect: { timeout: 12_000 },
  reporter: 'list',
  use: { baseURL: `http://127.0.0.1:${port}`, channel: process.env.TMC_E2E_CHANNEL ?? 'msedge', headless: true, trace: 'retain-on-failure' },
  webServer: {
    command: 'node scripts/e2e-server.mjs',
    url: `http://127.0.0.1:${port}/api/health`,
    reuseExistingServer: process.env.TMC_GUIDE_REUSE_SERVER === '1',
    timeout: 60_000,
    env: {
      TMC_E2E_PORT: String(port), TMC_E2E_ARDENT_PORT: String(port + 1),
      TMC_E2E_FIXTURE: 'engineering_session.log', TMC_E2E_SHIFT: 'true', TMC_E2E_ONLINE: 'false',
    },
  },
});
