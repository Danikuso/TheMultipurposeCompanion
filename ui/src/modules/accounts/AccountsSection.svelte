<script lang="ts">
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import CapiCard from './CapiCard.svelte';
  import SyncKeys from './SyncKeys.svelte';
  import type { AccountsStatusTopic } from './types';

  const status = $derived(topic<AccountsStatusTopic>('accounts.status'));
  const currentCmdrName = $derived(status?.current_cmdr?.name ?? null);
</script>

<div class="accounts-container">
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

  <SyncKeys status={status ?? null} />

  <CapiCard capi={status?.capi ?? null} />
</div>

<style>
  .accounts-container {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0;
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
    font-size: 26px;
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

  .mono {
    font-family: var(--font-mono, ui-monospace, monospace);
  }
</style>
