// @ts-nocheck -- utilidades de Node para Playwright (sin tipos de Node en el proyecto)
import { execSync } from 'node:child_process';
import { type Page } from '@playwright/test';

/** Abre la disposición por su nombre visible (botón de la barra lateral). */
export async function openLayout(page: Page, name: string) {
  await page.goto('/');
  await page.locator('button.nav-btn:not([data-module])').filter({ hasText: new RegExp(`^${name}$`) }).click();
}

/** Anexa una línea al diario en curso a través del servidor falso (solo escenarios con él). */
export async function appendJournal(fakePort: number, line: string) {
  const res = await fetch(`http://127.0.0.1:${fakePort}/__journal`, { method: 'POST', body: line });
  if (!res.ok) throw new Error(`__journal ${res.status}`);
}

export const basePort = Number(process.env.TMC_E2E_PORT ?? 47320);

/** Observa los marcos del WebSocket y recuerda si alguna vez llegó `eddn_connected: true`. */
export function watchEddnConnected(page: Page) {
  const state = { seen: false };
  page.on('websocket', (ws) =>
    ws.on('framereceived', (f) => {
      if (typeof f.payload === 'string' && /"eddn_connected"\s*:\s*true/.test(f.payload)) state.seen = true;
    }),
  );
  return state;
}

/** Texto del portapapeles del sistema (solo Windows; en otros sistemas, `null`). */
export function readClipboard(): string | null {
  if (process.platform !== 'win32') return null;
  return execSync('powershell -NoProfile -Command Get-Clipboard').toString().trim();
}

/** Restaura el texto del portapapeles (solo Windows). */
export function writeClipboard(text: string): void {
  if (process.platform !== 'win32') return;
  execSync('powershell -NoProfile -Command "$input | Set-Clipboard"', { input: text });
}

export function isWindows(): boolean {
  return process.platform === 'win32';
}
