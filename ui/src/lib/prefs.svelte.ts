import { diag } from './diag.svelte';

/** Preferencias de la UI persistidas en `config.toml` (vía `/api/config`). */
export const prefs = $state({ autoCompanion: true });

let queue: Promise<void> = Promise.resolve();

export function loadPrefs(cfg: { auto_companion?: unknown }): void {
  prefs.autoCompanion = typeof cfg.auto_companion === 'boolean' ? cfg.auto_companion : true;
}

/** Cambia el interruptor Auto-companion: optimista, con PUT completo y reversión si falla. */
export function setAutoCompanion(value: boolean): Promise<void> {
  const previous = prefs.autoCompanion;
  prefs.autoCompanion = value;
  queue = queue.then(async () => {
    try {
      const got = await fetch('/api/config');
      if (!got.ok) throw new Error('config-get-failed');
      const cfg = (await got.json()) as Record<string, unknown>;
      const res = await fetch('/api/config', {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ ...cfg, auto_companion: value }),
      });
      diag('ui.fetch.result', { kind: 'config-put', ok: res.ok, status: res.status });
      if (!res.ok) throw new Error('config-save-failed');
      loadPrefs((await res.json()) as { auto_companion?: unknown });
    } catch {
      prefs.autoCompanion = previous;
    }
  });
  return queue;
}
