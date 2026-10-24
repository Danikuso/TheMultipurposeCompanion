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
// F3: +5 comercio (Ardent/Spansh falsos en +11), +6 EDDN activo (+12, relay +14),
// +7 EDDN con servicios desactivados (relay +15), +8 misiones, +9 masacre, +10 turismo (+13).
const tradePort = port + 5;
const eddnOnPort = port + 6;
const eddnOffPort = port + 7;
const missionsPort = port + 8;
const massacrePort = port + 9;
const tourismPort = port + 10;

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
    { name: 'trade', testMatch: 'trade.spec.ts', use: { baseURL: `http://127.0.0.1:${tradePort}`, channel } },
    { name: 'eddn-on', testMatch: 'eddn-on.spec.ts', use: { baseURL: `http://127.0.0.1:${eddnOnPort}`, channel } },
    { name: 'eddn-off', testMatch: 'eddn-off.spec.ts', use: { baseURL: `http://127.0.0.1:${eddnOffPort}`, channel } },
    { name: 'missions', testMatch: 'missions.spec.ts', use: { baseURL: `http://127.0.0.1:${missionsPort}`, channel } },
    { name: 'massacre', testMatch: 'massacre.spec.ts', use: { baseURL: `http://127.0.0.1:${massacrePort}`, channel } },
    { name: 'tourism', testMatch: 'tourism.spec.ts', use: { baseURL: `http://127.0.0.1:${tourismPort}`, channel } },
    { name: 'engineering', testMatch: 'engineering.spec.ts', use: { baseURL: `http://127.0.0.1:${port + 16}`, channel }, workers: 1 },
  ],
  webServer: [
    server(port + 16, {
      TMC_E2E_FIXTURE: 'engineering_session.log', TMC_E2E_SHIFT: 'true',
      TMC_E2E_ONLINE: 'true', TMC_E2E_ARDENT_PORT: String(port + 17),
    }),
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
    server(tradePort, {
      TMC_E2E_FIXTURE: 'trade_session.log',
      TMC_E2E_ONLINE: 'true',
      TMC_E2E_SHIFT: 'true',
      TMC_E2E_MARKET: 'true',
      TMC_E2E_AUTOCOPY: 'true',
      TMC_E2E_ARDENT_PORT: String(port + 11),
    }),
    server(eddnOnPort, {
      TMC_E2E_FIXTURE: 'trade_session.log',
      TMC_E2E_ONLINE: 'true',
      TMC_E2E_SHIFT: 'true',
      TMC_E2E_MARKET: 'true',
      TMC_E2E_ARDENT_PORT: String(port + 12),
      TMC_E2E_RELAY_PORT: String(port + 14),
      TMC_E2E_RELAY_FILE: 'e2e_gold_buyer.jsonl',
    }),
    server(eddnOffPort, {
      TMC_E2E_FIXTURE: 'trade_session.log',
      TMC_E2E_SHIFT: 'true',
      TMC_E2E_MARKET: 'true',
      TMC_E2E_RELAY_PORT: String(port + 15),
      TMC_E2E_RELAY_FILE: 'e2e_gold_buyer.jsonl',
    }),
    server(missionsPort, { TMC_E2E_FIXTURE: 'missions_session.log', TMC_E2E_SHIFT: 'true' }),
    server(massacrePort, { TMC_E2E_FIXTURE: 'massacre_session.log', TMC_E2E_SHIFT: 'true' }),
    server(tourismPort, {
      TMC_E2E_FIXTURE: 'tourism_session.log',
      TMC_E2E_ONLINE: 'true',
      TMC_E2E_SHIFT: 'true',
      TMC_E2E_ARDENT_PORT: String(port + 13),
    }),
  ],
});
