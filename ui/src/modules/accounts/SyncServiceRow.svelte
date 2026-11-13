<script lang="ts">
  import { integer } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import {
    formatLastConfirmed,
    formatStateLabel,
    openSettingsAccounts,
    reasonText,
    SERVICE_NAMES,
    stateBadgeClass,
    stateIcon,
  } from './syncView';
  import type { EddnServiceStatus, KeyedServiceStatus, SyncServiceName } from './types';

  let {
    service,
    status,
    busy = false,
    onToggleEddn,
    onRetry,
    onOpenSettings = openSettingsAccounts,
  }: {
    service: SyncServiceName;
    status: KeyedServiceStatus | EddnServiceStatus | null;
    busy?: boolean;
    onToggleEddn?: (enabled: boolean) => void;
    onRetry?: (service: SyncServiceName) => void;
    onOpenSettings?: () => void;
  } = $props();

  const serviceIcon = $derived(service === 'eddn' ? '📡' : service === 'edsm' ? '🗺️' : '🌐');
  const serviceDesc = $derived(t(`accounts-${service}-desc`));
  const stateLabel = $derived(formatStateLabel(status));
  const reason = $derived(reasonText(status?.reason));
  const eddnStatus = $derived(service === 'eddn' ? (status as EddnServiceStatus | null) : null);
  const keyedStatus = $derived(service !== 'eddn' ? (status as KeyedServiceStatus | null) : null);
  const hasKey = $derived(service === 'eddn' ? true : Boolean(keyedStatus?.has_key));
  const showReasonLine = $derived(Boolean(reason));
</script>

<div class="sync-service-wrap" data-testid={`accounts-${service}-card`}>
  <div class="hud-list-row sync-row" data-testid={`sync-row-${service}`}>
    <div class="sync-row-main">
      <div class="sync-row-head">
        <span class="service-icon" aria-hidden="true">{serviceIcon}</span>
        <div class="service-title-col">
          <div class="service-name-line">
            <h4 class="service-name">{SERVICE_NAMES[service]}</h4>
            {#if service === 'inara'}
              <span
                class="badge badge-cyan mono whitelist-pill"
                data-testid="inara-whitelist-badge"
                title={t('accounts-inara-whitelist-tooltip')}
              >
                {t('accounts-inara-whitelist-badge')}
              </span>
            {/if}
            <span
              class="badge {stateBadgeClass(status?.state)}"
              data-testid={`${service}-state-pill`}
              title={reason ?? stateLabel}
            >
              {stateIcon(status?.state)} {stateLabel}
            </span>
          </div>
          <p class="sub service-desc">{serviceDesc}</p>
        </div>
      </div>

      {#if showReasonLine}
        <p class="sync-reason sub" data-testid={`${service}-reason-text`}>{reason}</p>
      {/if}

      {#if eddnStatus && (eddnStatus.paused_schemas?.length ?? 0) > 0}
        <p class="sync-reason sub mono" data-testid="eddn-paused-schemas">
          {t('accounts-eddn-paused-schemas', { schemas: eddnStatus.paused_schemas.join(', ') })}
        </p>
      {/if}
    </div>

    <div class="sync-metrics-grid">
      <div class="sync-metric-cell">
        <span class="metric-label">{t('accounts-sync-metric-sent')}</span>
        <span class="sync-metric-val mono cyan" data-testid={`${service}-metric-sent`}>
          {integer(status?.sent_total ?? 0)}
        </span>
      </div>
      <div class="sync-metric-cell">
        <span class="metric-label">{t('accounts-sync-metric-pending')}</span>
        <span class="sync-metric-val mono" data-testid={`${service}-metric-pending`}>
          {integer(status?.pending ?? 0)}
        </span>
      </div>
      <div class="sync-metric-cell">
        <span class="metric-label">{t('accounts-sync-metric-rejected')}</span>
        <span class="sync-metric-val mono" data-testid={`${service}-metric-rejected`}>
          {integer(status?.rejected_total ?? 0)}
        </span>
      </div>
      <div class="sync-metric-cell">
        <span class="metric-label">{t('accounts-sync-metric-last-confirmed')}</span>
        <span class="sync-metric-val mono" data-testid={`${service}-metric-last-confirmed`}>
          {formatLastConfirmed(status?.last_confirmed_at)}
        </span>
      </div>
    </div>

    <div class="sync-row-actions">
      {#if service === 'eddn'}
        <input
          type="checkbox"
          role="switch"
          class="hud-switch"
          checked={Boolean(eddnStatus?.enabled)}
          disabled={busy}
          onchange={(e) => onToggleEddn?.((e.currentTarget as HTMLInputElement).checked)}
          data-testid="eddn-switch"
          aria-label={t('accounts-eddn-switch')}
        />
      {/if}

      {#if status?.state === 'error'}
        <button
          type="button"
          class="hud-btn btn-sm"
          onclick={() => onRetry?.(service)}
          disabled={busy}
          data-testid={`${service}-retry-btn`}
        >
          {busy ? '...' : t('accounts-sync-retry-now')}
        </button>
      {/if}

      {#if service !== 'eddn' && !hasKey}
        <button
          type="button"
          class="hud-btn btn-sm"
          onclick={onOpenSettings}
          data-testid={`${service}-go-settings-btn`}
        >
          {t('accounts-sync-go-settings')}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .sync-service-wrap {
    width: 100%;
    min-width: 0;
  }

  .sync-row {
    display: grid;
    grid-template-columns: minmax(220px, 1.35fr) minmax(240px, 1.2fr) auto;
    align-items: center;
    gap: 12px 16px;
    padding: 12px 14px;
    margin-bottom: 0;
  }

  .sync-row-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .sync-row-head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 0;
  }

  .service-icon {
    font-size: 20px;
    line-height: 1.2;
    flex-shrink: 0;
  }

  .service-title-col {
    min-width: 0;
    flex: 1;
  }

  .service-name-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 8px;
    min-width: 0;
  }

  .service-name {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: var(--hud-text, #f0f4f8);
  }

  .service-desc {
    margin: 2px 0 0;
    font-size: 12px;
    line-height: 1.35;
    color: var(--hud-muted, #94a3b8);
  }

  .sync-reason {
    margin: 2px 0 0;
    font-size: 11px;
    line-height: 1.35;
    color: var(--hud-muted, #94a3b8);
    word-break: break-word;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
  }

  .badge-green { background: rgba(16, 185, 129, 0.16); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.35); }
  .badge-amber { background: rgba(245, 158, 11, 0.16); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.35); }
  .badge-red { background: rgba(239, 68, 68, 0.16); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.35); }
  .badge-cyan { background: rgba(0, 212, 178, 0.12); color: #00d4b2; border: 1px solid rgba(0, 212, 178, 0.35); }
  .badge-muted { background: rgba(148, 163, 184, 0.14); color: #cbd5e1; border: 1px solid rgba(148, 163, 184, 0.3); }

  .sync-metrics-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    min-width: 0;
  }

  .sync-metric-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 5px 8px;
    border-radius: 4px;
    background: rgba(10, 15, 25, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.06);
    min-width: 0;
  }

  .metric-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--hud-muted, #94a3b8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sync-metric-val {
    font-size: 13px;
    font-weight: 700;
    color: var(--hud-text, #f0f4f8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cyan { color: var(--hud-info, #38bdf8); }
  .mono { font-family: var(--font-mono, ui-monospace, monospace); }

  .sync-row-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    flex-shrink: 0;
  }

  .hud-switch {
    appearance: none;
    position: relative;
    width: 42px;
    height: 22px;
    border-radius: 999px;
    background: #1e293b;
    border: 1px solid rgba(148, 163, 184, 0.35);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 150ms ease, border-color 150ms ease;
  }

  .hud-switch::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #cbd5e1;
    transition: transform 150ms ease, background-color 150ms ease;
  }

  .hud-switch:checked {
    background: rgba(0, 212, 178, 0.25);
    border-color: #00d4b2;
  }

  .hud-switch:checked::before {
    transform: translateX(20px);
    background: #00d4b2;
  }

  @media (max-width: 960px) {
    .sync-row {
      grid-template-columns: 1fr;
    }

    .sync-row-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 540px) {
    .sync-metrics-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
