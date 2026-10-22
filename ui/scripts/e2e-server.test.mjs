import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const uiDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = path.resolve(uiDir, '../target/debug/tmc-headless.exe');
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function until(read, accept, timeout = 60_000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await read();
    if (accept(value)) return value;
    await delay(100);
  }
  assert.fail('Se agotó la espera del harness');
}

function processes(config) {
  const raw = execFileSync('powershell.exe', ['-NoProfile', '-Command',
    "Get-CimInstance Win32_Process -Filter \"Name = 'tmc-headless.exe'\" | Select-Object ProcessId,ParentProcessId,CommandLine | ConvertTo-Json -Compress"], { encoding: 'utf8', windowsHide: true });
  const rows = raw.trim() ? JSON.parse(raw) : [];
  return (Array.isArray(rows) ? rows : [rows]).filter((row) => row.CommandLine?.includes(config));
}

async function available(port) {
  const server = net.createServer();
  return new Promise((resolve) => {
    server.once('error', () => resolve(false));
    server.listen(port, '127.0.0.1', () => server.close(() => resolve(true)));
  });
}

test('missing prebuilt binary owns the real process, restarts SQLite and releases ports', {
  skip: process.platform !== 'win32', timeout: 180_000,
}, async () => {
  const port = 47916;
  const fakePort = port + 1;
  assert.equal(await available(port), true);
  assert.equal(await available(fakePort), true);
  const backup = path.join(path.dirname(binary), `tmc-headless-fallback-${process.pid}.exe`);
  const hadBinary = fs.existsSync(binary);
  if (hadBinary) fs.renameSync(binary, backup);
  let harness;
  let browser;
  let config;
  let output = '';
  try {
    harness = spawn(process.execPath, ['scripts/e2e-server.mjs'], {
      cwd: uiDir, windowsHide: true,
      env: { ...process.env, TMC_E2E_PORT: String(port), TMC_E2E_ARDENT_PORT: String(fakePort),
        TMC_E2E_FIXTURE: 'engineering_session.log', TMC_E2E_SHIFT: 'true', TMC_E2E_ONLINE: 'true' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    harness.stdout.on('data', (chunk) => { output += chunk; });
    harness.stderr.on('data', (chunk) => { output += chunk; });
    config = path.join(os.tmpdir(), `tmc-playwright-smoke-${harness.pid}`, 'config.toml');
    await until(async () => {
      if (harness.exitCode !== null) assert.fail(output);
      try { return (await fetch(`http://127.0.0.1:${port}/api/health`)).ok; } catch { return false; }
    }, Boolean, 120_000);
    const before = processes(config);
    assert.equal(before.length, 1);
    assert.equal(before[0].ParentProcessId, harness.pid, 'headless debe ser hijo directo del harness, sin Cargo intermedio');
    browser = await chromium.launch({ channel: 'msedge' });
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.locator('button.nav-btn:not([data-module])').filter({ hasText: /^Ingeniería$/ }).click();
    await page.getByTestId('module-MainEngines').click();
    await page.getByTestId('preview').click();
    await page.getByTestId('pin-goal').click();
    await page.locator('[data-goal-id]').waitFor();
    const goal = await page.locator('[data-goal-id]').getAttribute('data-goal-id');
    assert.equal((await fetch(`http://127.0.0.1:${fakePort}/__restart`, { method: 'POST' })).ok, true);
    const after = await until(() => processes(config), (rows) => rows.length === 1 && rows[0].ProcessId !== before[0].ProcessId);
    assert.equal(after[0].ParentProcessId, harness.pid);
    await until(async () => {
      try { return (await fetch(`http://127.0.0.1:${port}/api/health`)).ok; } catch { return false; }
    }, Boolean);
    await fetch(`http://127.0.0.1:${fakePort}/__journal`, { method: 'POST', body: JSON.stringify({
      timestamp: new Date().toISOString(), event: 'MaterialCollected', Name: 'iron', Category: 'Raw', Count: 0,
    }) });
    await page.reload();
    await page.locator('button.nav-btn:not([data-module])').filter({ hasText: /^Ingeniería$/ }).click();
    await page.locator(`[data-goal-id="${goal}"]`).waitFor();
    // Playwright cierra el árbol del servidor con taskkill en Windows.
    execFileSync('taskkill.exe', ['/pid', String(harness.pid), '/T', '/F'], { windowsHide: true });
    await until(() => processes(config), (rows) => rows.length === 0);
    assert.equal(await available(port), true);
    assert.equal(await available(fakePort), true);
  } finally {
    try {
      await browser?.close();
      // Solo procesos de esta configuración sintética, incluso si el harness antiguo dejó huérfanos.
      if (config) for (const row of processes(config)) {
        try { process.kill(row.ProcessId); } catch {}
      }
      harness?.kill();
      await until(() => config ? processes(config) : [], (rows) => rows.length === 0);
    } finally {
      if (fs.existsSync(binary)) fs.unlinkSync(binary);
      if (hadBinary) fs.renameSync(backup, binary);
    }
  }
});
