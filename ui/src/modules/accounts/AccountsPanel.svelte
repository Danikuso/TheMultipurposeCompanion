<script lang="ts">
  import Panel from '../../shell/Panel.svelte';
  import { integer } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { action, topic } from '../../lib/ws.svelte';
  import CapiCard from './CapiCard.svelte';
  import SyncDetails from './SyncDetails.svelte';
  import SyncGuide from './SyncGuide.svelte';
  import SyncServiceRow from './SyncServiceRow.svelte';
  import { openSettingsAccounts, type KeyedService } from './syncView';
  import type { AccountsStatusTopic, SyncServiceName } from './types';

  const status = $derived(topic<AccountsStatusTopic>('accounts.status'));
  let busyService = $state<SyncServiceName | null>(null);

  const currentCmdrName = $derived(status?.current_cmdr?.name ?? null);

  const missingServices = $derived.by<KeyedService[]>(() => {
    if (!status) return [];
    const list: KeyedService[] = [];
    if (!status.inara?.has_key) list.push('inara');
    if (!status.edsm?.has_key) list.push('edsm');
    return list;
  });

  const hasOtherCmdrsPending = $derived.by(() => {
    const o = status?.other_cmdrs_pending;
    if (!o) return false;
    return o.inara + o.edsm + o.eddn > 0;
  });

  async function toggleEddn(nextEnabled: boolean) {
    busyService = 'eddn';
    try {
      await action('accounts.eddn.set_enabled', { enabled: nextEnabled });
    } finally {
      busyService = null;
    }
  }

  async function retryNow(serviceId: SyncServiceName) {
    busyService = serviceId;
    try {
      await action('accounts.sync_now');
    } finally {
      busyService = null;
    }
  }
</script>

<Panel titleKey="panel-accounts" span="full">
  <div class="accounts-panel-body">
    <div class="privacy-banner glass-card" data-testid="accounts-privacy-banner">
      <div class="shield-icon" aria-hidden="true">🛡️</div>
      <div class="banner-content">
        <div class="badges-wrap">
          <span class="badge badge-green mono">{t('accounts-title')}</span>
          {#if currentCmdrName}
            <span class="cmdr-pill mono" data-testid="accounts-active-cmdr">CMDR {currentCmdrName}</span>
          {/if}
        </div>
        <p class="privacy-text">{t('accounts-companion-privacy-notice')}</p>
      </div>
    </div>

    <div class="sync-rows-list">
      <SyncServiceRow
        service="eddn"
        status={status?.eddn ?? null}
        busy={busyService === 'eddn'}
        onToggleEddn={toggleEddn}
        onRetry={retryNow}
        onOpenSettings={openSettingsAccounts}
      />
      <SyncServiceRow
        service="edsm"
        status={status?.edsm ?? null}
        busy={busyService === 'edsm'}
        onRetry={retryNow}
        onOpenSettings={openSettingsAccounts}
      />
      <SyncServiceRow
        service="inara"
        status={status?.inara ?? null}
        busy={busyService === 'inara'}
        onRetry={retryNow}
        onOpenSettings={openSettingsAccounts}
      />
    </div>

    {#if hasOtherCmdrsPending && status?.other_cmdrs_pending}
      <p class="other-cmdrs-summary sub mono" data-testid="other-cmdrs-pending-summary">
        {t('accounts-sync-other-cmdrs', {
          inara: integer(status.other_cmdrs_pending.inara),
          edsm: integer(status.other_cmdrs_pending.edsm),
          eddn: integer(status.other_cmdrs_pending.eddn),
        })}
      </p>
    {/if}

    <SyncGuide {missingServices} onOpenSettings={openSettingsAccounts} />

    <CapiCard capi={status?.capi ?? null} />
  </div>

  {#snippet details()}
    <SyncDetails status={status ?? null} />
  {/snippet}
</Panel>

<style>
  .accounts-panel-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }

  .glass-card {
    background: var(--hud-surface, rgba(14, 22, 33, 0.95));
    border: 1px solid var(--glass-border, rgba(0, 212, 178, 0.2));
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .privacy-banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    border-left: 3px solid #00d4b2;
    background: rgba(0, 212, 178, 0.05);
  }

  .shield-icon {
    font-size: 24px;
    line-height: 1;
    flex-shrink: 0;
  }

  .banner-content {
    min-width: 0;
    flex: 1;
  }

  .badges-wrap {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .badge-green {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.16);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.35);
  }

  .cmdr-pill {
    font-size: 11px;
    font-weight: 700;
    color: #00d4b2;
    background: rgba(0, 212, 178, 0.1);
    border: 1px solid rgba(0, 212, 178, 0.3);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .privacy-text {
    font-size: 12px;
    color: var(--text-dim, #8fa3b8);
    margin: 4px 0 0;
    line-height: 1.4;
  }

  .sync-rows-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .other-cmdrs-summary {
    margin: 0;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(56, 189, 248, 0.06);
    border: 1px solid rgba(56, 189, 248, 0.25);
    font-size: 12px;
    color: var(--hud-text, #cbd5e1);
  }

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
  }
</style>
