<script lang="ts">
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  interface LocationTopic {
    system?: string;
    body?: string | null;
    station?: string | null;
    mode?: string;
  }

  const MODE_KEY_MAP: Record<string, string> = {
    Docked: 'mode-docked',
    Landed: 'mode-landed',
    NormalSpace: 'mode-normal-space',
    Supercruise: 'mode-supercruise',
    Hyperspace: 'mode-hyperspace',
    OnFoot: 'mode-on-foot',
    InSrv: 'mode-in-srv',
    InFighter: 'mode-in-fighter',
    InTaxi: 'mode-in-taxi',
    Multicrew: 'mode-multicrew',
    Unknown: 'mode-unknown',
  };

  const loc = $derived(topic<LocationTopic>('core.location'));
  const modeKey = $derived(MODE_KEY_MAP[loc?.mode ?? 'Unknown'] ?? 'mode-unknown');
</script>

<Panel titleKey="panel-location">
  {#if !loc || !loc.system}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="loc-rows">
      <div class="row">
        <span class="label">{t('field-system')}</span>
        <strong class="mono">{loc.system}</strong>
      </div>
      <div class="row">
        <span class="label">{t('field-body')}</span>
        <span>{loc.body || '—'}</span>
      </div>
      <div class="row">
        <span class="label">{t('field-station')}</span>
        <span>{loc.station || '—'}</span>
      </div>
      <div class="row">
        <span class="label">{t('panel-status')}</span>
        <span class="mode-badge">{t(modeKey)}</span>
      </div>
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .loc-rows {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .label {
    color: var(--hud-muted);
  }

  .mono {
    font-family: var(--font-mono);
    color: var(--hud-primary);
  }

  .mode-badge {
    font-size: 0.8rem;
    padding: 0.1rem 0.45rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border);
    color: var(--hud-secondary);
  }
</style>
