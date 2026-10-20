<script lang="ts">
  import { t } from '../../lib/i18n';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { AlertsTopic, MissionAlertItem } from './types';

  const a = $derived(topic<AlertsTopic>('missions.alerts'));
  const alerts = $derived(a?.alerts ?? []);

  function text(x: MissionAlertItem): string {
    const k = x.kind;
    const name = x.name ?? '—';
    if (k === 'ExpiringSoon') return t('missions-alert-expiring', { name });
    if (k === 'Failed') return t('missions-alert-failed', { name });
    if ('Redirected' in k) {
      const r = k.Redirected;
      return t('missions-alert-redirected', { destination: r.station ? `${r.system} · ${r.station}` : r.system });
    }
    return t('missions-alert-cargo-missing', { tons: k.CargoMissing.missing });
  }

  function dismiss(id: number) {
    void action('missions.dismiss_alert', { mission_id: id }).catch(() => {});
  }
</script>

<Panel titleKey="panel-missions-alerts" span={1}>
  {#if alerts.length === 0}
    <p class="muted" data-testid="missions-alerts-empty">{t('missions-no-alerts')}</p>
  {:else}
    <ul>
      {#each alerts as x (`${x.mission_id}|${x.at}|${x.bucket}|${typeof x.kind === 'string' ? x.kind : Object.keys(x.kind)[0]}`)}
        <li data-testid="missions-alert">
          <span class="warn" aria-hidden="true">⚠</span>
          <span>{text(x)}</span>
          <button type="button" onclick={() => dismiss(x.mission_id)}>{t('missions-dismiss')}</button>
        </li>
      {/each}
    </ul>
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; gap: 0.5rem; align-items: baseline; margin: 0.25rem 0; }
  li button { margin-left: auto; }
  .warn { color: var(--hud-warning); }
</style>
