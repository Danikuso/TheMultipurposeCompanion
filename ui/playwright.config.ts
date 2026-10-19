import { defineConfig } from '@playwright/test';

const port = Number(process.env.TMC_E2E_PORT ?? 47320);
// Puertos: TMC_E2E_PORT (47320 por defecto) +0 exploración, +1 combate, +2 minería online,
// +3 minería offline, +4 Ardent falso (solo 127.0.0.1). Si 47320 está ocupado (p. ej. TMC.exe),
// usar otro: TMC_E2E_PORT=47420 npx playwright test.
// Servidores extra por escenario (cada uno con su fixture de diario).
const combatPort = port + 1;
const miningOnPort = port + 2;
const miningOffPort = port + 3;
const ardentPort = port + 4;

const server = (p: number, env: Record<string, string>) => ({
  command: 'node scripts/e2e-server.mjs',
  url: `http://127.0.0.1:${p}/api/health`,
  reuseExistingServer: false,
  timeout: 60_000,
  env: { ...(process.env as Record<string, string>), TMC_E2E_PORT: String(p), ...env },
});

const channel = process.platform === 'win32' ? 'msedge' : undefined;

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  projects: [
    { name: 'explo', testMatch: 'smoke.spec.ts', use: { baseURL: `http://127.0.0.1:${port}`, channel } },
    { name: 'combat', testMatch: 'combat.spec.ts', use: { baseURL: `http://127.0.0.1:${combatPort}`, channel } },
    { name: 'mining-on', testMatch: 'mining-on.spec.ts', use: { baseURL: `http://127.0.0.1:${miningOnPort}`, channel } },
    { name: 'mining-off', testMatch: 'mining-off.spec.ts', use: { baseURL: `http://127.0.0.1:${miningOffPort}`, channel } },
  ],
  webServer: [
    {
      command: 'node scripts/e2e-server.mjs',
      url: `http://127.0.0.1:${port}/api/health`,
      reuseExistingServer: false,
      timeout: 30_000,
    },
    server(combatPort, { TMC_E2E_FIXTURE: 'combat_session.log' }),
    server(miningOnPort, {
      TMC_E2E_FIXTURE: 'mining_session.log',
      TMC_E2E_ONLINE: 'true',
      TMC_E2E_SHIFT: 'true',
      TMC_E2E_ARDENT_PORT: String(ardentPort),
    }),
    server(miningOffPort, { TMC_E2E_FIXTURE: 'mining_session.log' }),
  ],
});
