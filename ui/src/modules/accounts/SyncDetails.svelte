<script lang="ts">
  import { integer } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import {
    adaptiveListBox,
    errorClassText,
    formatLastConfirmed,
    formatRetryTime,
    SERVICE_NAMES,
  } from './syncView';
  import type {
    AccountsStatusTopic,
    RecentRejection,
    SyncServiceName,
    SyncServiceStatus,
  } from './types';

  let { status }: { status: AccountsStatusTopic | null } = $props();

  const SERVICES: SyncServiceName[] = ['eddn', 'edsm', 'inara'];

  function serviceStatus(svc: SyncServiceName): SyncServiceStatus | null {
    if (!status) return null;
    return status[svc];
  }

  const totalRecovered = $derived(
    (status?.eddn?.recovered_session ?? 0) +
      (status?.edsm?.recovered_session ?? 0) +
      (status?.inara?.recovered_session ?? 0)
  );

  const pausedSchemas = $derived(status?.eddn?.paused_schemas ?? []);
  const commanders = $derived(status?.commanders ?? []);

  type TaggedRejection = RecentRejection & { service: SyncServiceName };

  const recentRejections = $derived.by<TaggedRejection[]>(() => {
    const out: TaggedRejection[] = [];
    for (const svc of SERVICES) {
      const list = serviceStatus(svc)?.recent_rejections ?? [];
      for (const item of list) {
        out.push({ ...item, service: svc });
      }
    }
    out.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
    return out.slice(0, 20);
  });

  const rejectionsBox = $derived(adaptiveListBox(recentRejections.length, 36, 6));
  const cmdrsBox = $derived(adaptiveListBox(commanders.length, 34, 5));

  let privacyScrollEl = $state<HTMLDivElement | null>(null);
  let privacyStickyEl = $state<HTMLDivElement | null>(null);
  let privacyNeedsHScroll = $state(false);
  let privacyScrollWidth = $state(0);

  $effect(() => {
    const el = privacyScrollEl;
    if (!el) return;
    const measure = () => {
      privacyScrollWidth = el.scrollWidth;
      privacyNeedsHScroll = el.scrollWidth > el.clientWidth + 1;
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  });

  function syncPrivacyFromContent() {
    if (privacyScrollEl && privacyStickyEl) {
      privacyStickyEl.scrollLeft = privacyScrollEl.scrollLeft;
    }
  }

  function syncPrivacyFromSticky() {
    if (privacyScrollEl && privacyStickyEl) {
      privacyScrollEl.scrollLeft = privacyStickyEl.scrollLeft;
    }
  }
</script>

<div class="sync-details" data-testid="sync-details-section">
  <div class="details-warning-banner" data-testid="details-duplicate-warning">
    <span aria-hidden="true">⚠️</span>
    <span>{t('accounts-sync-duplicate-warning')}</span>
  </div>

  <div class="details-header-row">
    <h4 class="details-heading">{t('accounts-sync-details-breakdown-title')}</h4>
    <span class="recovered-pill mono" data-testid="details-recovered-total">
      {t('accounts-sync-details-recovered')}: {integer(totalRecovered)}
    </span>
  </div>

  <div class="breakdown-grid">
    {#each SERVICES as svc (svc)}
      {@const st = serviceStatus(svc)}
      {@const nextRetry = formatRetryTime(st?.next_attempt_at)}
      <div class="breakdown-card" data-testid={`details-${svc}-breakdown`}>
        <div class="breakdown-head">
          <span class="breakdown-name">{SERVICE_NAMES[svc]}</span>
          {#if nextRetry}
            <span class="next-retry mono">
              {t('accounts-sync-details-next-attempt')}: {nextRetry}
            </span>
          {/if}
        </div>
        <div class="breakdown-metrics">
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-metric-pending')}</span>
            <span class="bm-val mono">{integer(st?.pending ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-metric-sent')}</span>
            <span class="bm-val mono cyan">{integer(st?.sent_total ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-details-sent-session')}</span>
            <span class="bm-val mono">{integer(st?.sent_session ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-metric-rejected')}</span>
            <span class="bm-val mono">{integer(st?.rejected_total ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-details-rejected-session')}</span>
            <span class="bm-val mono">{integer(st?.rejected_session ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-details-expired')}</span>
            <span class="bm-val mono">{integer(st?.expired_total ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-details-omitted')}</span>
            <span class="bm-val mono">{integer(st?.omitted_total ?? 0)}</span>
          </div>
          <div class="bm-cell">
            <span class="bm-lbl">{t('accounts-sync-details-recovered')}</span>
            <span class="bm-val mono">{integer(st?.recovered_session ?? 0)}</span>
          </div>
        </div>
      </div>
    {/each}
  </div>

  <p class="discard-note sub">{t('accounts-sync-details-discard-note')}</p>

  {#if pausedSchemas.length > 0}
    <div class="paused-schemas-box" data-testid="details-eddn-paused-schemas">
      <h4 class="details-heading">{t('accounts-sync-details-paused-schemas-title')}</h4>
      <div class="schemas-chips">
        {#each pausedSchemas as schema (schema)}
          <span class="schema-chip mono">{schema}</span>
        {/each}
      </div>
    </div>
  {/if}

  {#if commanders.length > 0}
    <div class="details-block">
      <h4 class="details-heading">{t('accounts-sync-details-cmdrs-title')}</h4>
      <div
        class="adaptive-list"
        data-testid="details-cmdrs-list"
        style:height="{cmdrsBox.heightPx}px"
        style:overflow-y={cmdrsBox.overflowY}
      >
        {#each commanders as cmdr (cmdr.cmdr_ref)}
          {@const isActive = status?.current_cmdr?.cmdr_ref === cmdr.cmdr_ref}
          <div class="cmdr-pending-row" class:is-active={isActive} data-testid="details-cmdr-pending-row">
            <span class="cmdr-name bold">CMDR {cmdr.name}</span>
            <span class="cmdr-counts mono">
              Inara {integer(cmdr.inara_pending)} · EDSM {integer(cmdr.edsm_pending)} · EDDN {integer(cmdr.eddn_pending)}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="details-block">
    <h4 class="details-heading">
      {t('accounts-sync-details-rejections-title', { count: recentRejections.length })}
    </h4>
    {#if recentRejections.length === 0}
      <p class="sub">{t('accounts-sync-details-no-rejections')}</p>
    {:else}
      <div
        class="adaptive-list"
        data-testid="sync-rejections-list"
        style:height="{rejectionsBox.heightPx}px"
        style:overflow-y={rejectionsBox.overflowY}
      >
        {#each recentRejections as rej, idx (`${rej.service}-${rej.at}-${rej.kind}-${idx}`)}
          <div class="rejection-row" title={rej.hint ?? ''} data-testid="sync-rejection-row">
            <span class="rej-svc mono">{SERVICE_NAMES[rej.service]}</span>
            <span class="rej-kind mono">{rej.kind}</span>
            <span class="rej-class">{errorClassText(rej.class)}</span>
            <span class="rej-code mono">{rej.code}</span>
            <span class="rej-time mono">{formatLastConfirmed(rej.at)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="details-block">
    <h4 class="details-heading">{t('accounts-sync-details-privacy-title')}</h4>
    <div
      class="privacy-table-wrap"
      bind:this={privacyScrollEl}
      onscroll={syncPrivacyFromContent}
    >
      <table class="privacy-table" data-testid="details-privacy-table">
        <thead>
          <tr>
            <th>{t('accounts-sync-details-privacy-service')}</th>
            <th>{t('accounts-sync-details-privacy-receives')}</th>
            <th>{t('accounts-sync-details-privacy-never')}</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td class="mono bold">EDDN</td>
            <td>{t('accounts-sync-privacy-eddn-in')}</td>
            <td>{t('accounts-sync-privacy-eddn-out')}</td>
          </tr>
          <tr>
            <td class="mono bold">EDSM</td>
            <td>{t('accounts-sync-privacy-edsm-in')}</td>
            <td>{t('accounts-sync-privacy-edsm-out')}</td>
          </tr>
          <tr>
            <td class="mono bold">Inara</td>
            <td>{t('accounts-sync-privacy-inara-in')}</td>
            <td>{t('accounts-sync-privacy-inara-out')}</td>
          </tr>
        </tbody>
      </table>
    </div>
    {#if privacyNeedsHScroll}
      <div
        class="sticky-hscroll"
        bind:this={privacyStickyEl}
        onscroll={syncPrivacyFromSticky}
      >
        <div class="sticky-hscroll-spacer" style:width="{privacyScrollWidth}px"></div>
      </div>
    {/if}
  </div>
</div>

<style>
  .sync-details {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    min-width: 0;
  }

  .details-warning-banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.35);
    color: #fde68a;
    font-size: 12px;
    line-height: 1.4;
  }

  .details-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
  }

  .details-heading {
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--hud-primary, #ff7100);
  }

  .recovered-pill {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.3);
    color: #38bdf8;
  }

  .breakdown-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 10px;
  }

  .breakdown-card {
    background: rgba(10, 15, 25, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .breakdown-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px;
  }

  .breakdown-name {
    font-size: 13px;
    font-weight: 700;
    color: #f0f4f8;
  }

  .next-retry {
    font-size: 11px;
    color: #fbbf24;
  }

  .breakdown-metrics {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
  }

  .bm-cell {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    font-size: 11px;
    padding: 3px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.03);
    min-width: 0;
  }

  .bm-lbl {
    color: var(--hud-muted, #94a3b8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bm-val {
    color: #e2e8f0;
    font-weight: 700;
    flex-shrink: 0;
  }

  .discard-note {
    margin: 0;
    font-size: 11px;
    color: var(--hud-muted, #94a3b8);
    line-height: 1.4;
  }

  .paused-schemas-box, .details-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .schemas-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .schema-chip {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(239, 68, 68, 0.14);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fca5a5;
  }

  .adaptive-list {
    display: flex;
    flex-direction: column;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(10, 15, 25, 0.55);
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) transparent;
  }

  .cmdr-pending-row, .rejection-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 12px;
    box-sizing: border-box;
    flex-shrink: 0;
  }

  .cmdr-pending-row { height: 34px; }
  .rejection-row { height: 36px; }
  .cmdr-pending-row:last-child, .rejection-row:last-child { border-bottom: none; }

  .cmdr-pending-row.is-active {
    background: rgba(0, 212, 178, 0.08);
  }

  .cmdr-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cmdr-counts {
    font-size: 11px;
    color: #cbd5e1;
    flex-shrink: 0;
  }

  .rej-svc {
    font-size: 11px;
    font-weight: 700;
    color: #38bdf8;
    min-width: 44px;
    flex-shrink: 0;
  }

  .rej-kind {
    font-size: 11px;
    color: #e2e8f0;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rej-class {
    font-size: 11px;
    color: #fbbf24;
    flex-shrink: 0;
  }

  .rej-code {
    font-size: 11px;
    color: #f87171;
    flex-shrink: 0;
  }

  .rej-time {
    font-size: 11px;
    color: #94a3b8;
    flex-shrink: 0;
  }

  .privacy-table-wrap {
    width: 100%;
    overflow-x: auto;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(10, 15, 25, 0.55);
    scrollbar-width: thin;
  }

  .privacy-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .privacy-table th {
    position: sticky;
    top: 0;
    z-index: 2;
    text-align: left;
    padding: 7px 10px;
    background: rgba(15, 23, 42, 0.95);
    color: var(--hud-muted, #94a3b8);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .privacy-table td {
    padding: 7px 10px;
    color: #cbd5e1;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    vertical-align: top;
    line-height: 1.35;
  }

  .privacy-table tr:last-child td {
    border-bottom: none;
  }

  .sticky-hscroll {
    position: sticky;
    bottom: 0;
    z-index: 10;
    overflow-x: auto;
    overflow-y: hidden;
    height: 14px;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.55) transparent;
  }

  .sticky-hscroll-spacer {
    height: 1px;
  }

  .mono { font-family: var(--font-mono, ui-monospace, monospace); }
  .cyan { color: var(--hud-info, #38bdf8); }
  .bold { font-weight: 700; }
  .sub { font-size: 12px; color: var(--hud-muted, #94a3b8); margin: 0; }
</style>
