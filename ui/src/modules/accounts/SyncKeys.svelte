<script lang="ts">
  import { integer } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { action } from '../../lib/ws.svelte';
  import SyncServiceRow from './SyncServiceRow.svelte';
  import {
    formatKeyHint,
    formatLastConfirmed,
    formatStateLabel,
    reasonText,
    SERVICE_NAMES,
    stateBadgeClass,
    stateIcon,
    type KeyedService,
  } from './syncView';
  import type { AccountsStatusTopic, KeyedServiceStatus, SyncServiceName } from './types';

  let { status }: { status: AccountsStatusTopic | null } = $props();

  const KEYED_SERVICES: KeyedService[] = ['edsm', 'inara'];

  let selectedCmdrRef = $state<string | null>(null);
  let forgetConfirmOpen = $state(false);

  let keyInputs = $state<Record<KeyedService, string>>({ edsm: '', inara: '' });
  let showKey = $state<Record<KeyedService, boolean>>({ edsm: false, inara: false });
  let savingService = $state<KeyedService | null>(null);
  let pendingDeleteConfirm = $state<KeyedService | null>(null);
  let feedback = $state<Record<KeyedService, { tone: 'green' | 'red' | 'amber'; text: string } | null>>({
    edsm: null,
    inara: null,
  });
  let unassignedTargets = $state<Record<KeyedService, string>>({ edsm: '', inara: '' });
  let reassignTargets = $state<Record<KeyedService, string>>({ edsm: '', inara: '' });
  let busyAction = $state<SyncServiceName | null>(null);

  const commanders = $derived(status?.commanders ?? []);
  const effectiveCmdrRef = $derived(
    selectedCmdrRef && commanders.some((c) => c.cmdr_ref === selectedCmdrRef)
      ? selectedCmdrRef
      : (status?.current_cmdr?.cmdr_ref ?? commanders[0]?.cmdr_ref ?? null)
  );
  const selectedCmdr = $derived(
    commanders.find((c) => c.cmdr_ref === effectiveCmdrRef) ?? null
  );
  const isSelectedCurrent = $derived(
    effectiveCmdrRef != null && effectiveCmdrRef === (status?.current_cmdr?.cmdr_ref ?? null)
  );
  const unassignedList = $derived(
    (status?.unassigned_keys ?? []).filter(
      (s): s is KeyedService => s === 'inara' || s === 'edsm'
    )
  );

  $effect(() => {
    if (commanders.length > 0 && (!selectedCmdrRef || !commanders.some((c) => c.cmdr_ref === selectedCmdrRef))) {
      selectedCmdrRef = status?.current_cmdr?.cmdr_ref ?? commanders[0]?.cmdr_ref ?? null;
    }
    const defaultRef = commanders[0]?.cmdr_ref ?? '';
    for (const svc of KEYED_SERVICES) {
      if (defaultRef && (!unassignedTargets[svc] || !commanders.some((c) => c.cmdr_ref === unassignedTargets[svc]))) {
        unassignedTargets[svc] = effectiveCmdrRef ?? defaultRef;
      }
      if (!reassignTargets[svc] || !commanders.some((c) => c.cmdr_ref === reassignTargets[svc])) {
        reassignTargets[svc] =
          commanders.find((c) => c.cmdr_ref !== effectiveCmdrRef)?.cmdr_ref ?? effectiveCmdrRef ?? defaultRef;
      }
    }
  });

  function serviceStatus(svc: KeyedService): KeyedServiceStatus | null {
    return status?.[svc] ?? null;
  }

  function hasKeyForSelected(svc: KeyedService): boolean {
    if (selectedCmdr) {
      return svc === 'edsm' ? selectedCmdr.edsm_key : selectedCmdr.inara_key;
    }
    return Boolean(serviceStatus(svc)?.has_key);
  }

  function hintForSelected(svc: KeyedService): string | null {
    if (!isSelectedCurrent && selectedCmdr) return null;
    return formatKeyHint(serviceStatus(svc)?.key_hint);
  }

  async function saveKey(svc: KeyedService) {
    const cmdrRef = effectiveCmdrRef;
    const rawKey = keyInputs[svc].trim();
    if (!rawKey || !cmdrRef) return;
    savingService = svc;
    feedback[svc] = null;
    try {
      const res = (await action('accounts.key.save', {
        service: svc,
        cmdr_ref: cmdrRef,
        api_key: rawKey,
      })) as { ok?: boolean; verified?: boolean; state?: string } | undefined;
      if (res?.state === 'rejected') {
        feedback[svc] = { tone: 'red', text: t('accounts-key-verify-rejected') };
      } else if (res?.verified || res?.state === 'active') {
        keyInputs[svc] = '';
        feedback[svc] = { tone: 'green', text: t('accounts-key-verify-active') };
      } else {
        keyInputs[svc] = '';
        feedback[svc] = { tone: 'amber', text: t('accounts-key-verify-unverified') };
      }
    } finally {
      savingService = null;
    }
  }

  async function confirmDeleteKey(svc: KeyedService) {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    savingService = svc;
    pendingDeleteConfirm = null;
    feedback[svc] = null;
    try {
      const res = (await action('accounts.key.delete', {
        service: svc,
        cmdr_ref: cmdrRef,
      })) as { discarded?: number } | undefined;
      feedback[svc] = {
        tone: 'amber',
        text: t('accounts-key-pending-discarded', { count: res?.discarded ?? 0 }),
      };
    } finally {
      savingService = null;
    }
  }

  async function assignKey(svc: KeyedService, cmdrRef: string) {
    if (!cmdrRef) return;
    await action('accounts.key.assign', { service: svc, cmdr_ref: cmdrRef });
  }

  async function confirmOwner(svc: KeyedService) {
    if (!effectiveCmdrRef) return;
    await action('accounts.key.confirm', { service: svc, cmdr_ref: effectiveCmdrRef });
  }

  async function confirmForgetCommander() {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    forgetConfirmOpen = false;
    await action('accounts.cmdr.forget', { cmdr_ref: cmdrRef });
    if (selectedCmdrRef === cmdrRef) {
      selectedCmdrRef = null;
    }
  }

  async function toggleEddn(nextEnabled: boolean) {
    busyAction = 'eddn';
    try {
      await action('accounts.eddn.set_enabled', { enabled: nextEnabled });
    } finally {
      busyAction = null;
    }
  }

  async function retryNow(serviceId: SyncServiceName) {
    busyAction = serviceId;
    try {
      await action('accounts.sync_now');
    } finally {
      busyAction = null;
    }
  }
</script>

<div class="sync-keys-root">
  {#if commanders.length > 0}
    <div class="cmdr-selector-bar glass-card" data-testid="accounts-cmdr-bar">
      <div class="cmdr-selector-row">
        <label class="cmdr-selector-label" for="accounts-cmdr-select">
          <span class="sub bold">{t('accounts-cmdr-selector-label')}</span>
        </label>
        <div class="cmdr-selector-controls">
          <select
            id="accounts-cmdr-select"
            class="glass-select"
            data-testid="accounts-cmdr-select"
            bind:value={selectedCmdrRef}
            onchange={() => {
              forgetConfirmOpen = false;
              pendingDeleteConfirm = null;
              feedback = { edsm: null, inara: null };
            }}
          >
            {#each commanders as cmdr (cmdr.cmdr_ref)}
              <option value={cmdr.cmdr_ref}>CMDR {cmdr.name}</option>
            {/each}
          </select>
          {#if effectiveCmdrRef && !isSelectedCurrent}
            <button
              type="button"
              class="btn-outline btn-compact"
              onclick={() => (forgetConfirmOpen = true)}
              data-testid="accounts-cmdr-forget-btn"
            >
              {t('accounts-cmdr-forget')}
            </button>
          {/if}
        </div>
      </div>

      {#if forgetConfirmOpen && selectedCmdr}
        <div class="confirm-banner" data-testid="accounts-cmdr-forget-confirm-box">
          <p class="sub">{t('accounts-cmdr-forget-confirm', { cmdr: selectedCmdr.name })}</p>
          <div class="form-actions">
            <button
              type="button"
              class="action-btn btn-danger"
              onclick={confirmForgetCommander}
              data-testid="accounts-cmdr-forget-confirm-btn"
            >
              {t('accounts-ui-confirm')}
            </button>
            <button
              type="button"
              class="action-btn btn-secondary"
              onclick={() => (forgetConfirmOpen = false)}
              data-testid="accounts-cmdr-forget-cancel-btn"
            >
              {t('accounts-ui-cancel')}
            </button>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <p class="sub cmdr-empty-note">{t('accounts-cmdr-none')}</p>
  {/if}

  {#if unassignedList.length > 0 && commanders.length > 0}
    <div class="sync-callout callout-amber glass-card" data-testid="unassigned-keys-banner">
      {#each unassignedList as svc (svc)}
        <div class="unassigned-row">
          <p class="sub">{t('accounts-key-unassigned-service-banner', { service: SERVICE_NAMES[svc] })}</p>
          <div class="unassigned-controls">
            <select
              class="glass-select"
              bind:value={unassignedTargets[svc]}
              data-testid={`unassigned-${svc}-select`}
            >
              {#each commanders as cmdr (cmdr.cmdr_ref)}
                <option value={cmdr.cmdr_ref}>CMDR {cmdr.name}</option>
              {/each}
            </select>
            <button
              type="button"
              class="action-btn"
              onclick={() => assignKey(svc, unassignedTargets[svc])}
              data-testid={`unassigned-${svc}-assign-btn`}
            >
              {t('accounts-key-unassigned-assign')}
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <SyncServiceRow
    service="eddn"
    status={status?.eddn ?? null}
    busy={busyAction === 'eddn'}
    onToggleEddn={toggleEddn}
    onRetry={retryNow}
  />

  <div class="keyed-services-grid">
    {#each KEYED_SERVICES as svc (svc)}
      {@const svcStatus = serviceStatus(svc)}
      {@const hint = hintForSelected(svc)}
      {@const hasKey = hasKeyForSelected(svc)}
      {@const needsAutoConfirm = selectedCmdr?.key_assignment === 'auto_pending_confirmation' && hasKey}
      {@const reason = reasonText(svcStatus?.reason)}
      <div class="hud-card glass-card" data-testid={`accounts-${svc}-card`}>
        <div class="card-head">
          <div class="service-identity">
            <span class="service-icon" aria-hidden="true">{svc === 'edsm' ? '🌌' : '🎖️'}</span>
            <div class="service-titles">
              <h4 class="service-name">{t(`accounts-${svc}-title`)}</h4>
              <p class="sub">{t(`accounts-${svc}-desc`)}</p>
            </div>
          </div>
        </div>

        <div class="card-body">
          <div class="status-row">
            <div class="badges-wrap">
              <span
                class="badge {stateBadgeClass(svcStatus?.state)}"
                data-testid={`${svc}-state-pill`}
                title={reason ?? formatStateLabel(svcStatus)}
              >
                {stateIcon(svcStatus?.state)} {formatStateLabel(svcStatus)}
              </span>
              {#if svc === 'inara'}
                <span
                  class="badge badge-green"
                  data-testid="inara-whitelist-badge"
                  title={t('accounts-inara-whitelist-tooltip')}
                >
                  🛡️ {t('accounts-inara-whitelist-badge')}
                </span>
              {/if}
            </div>
            {#if hint}
              <span class="cmdr-pill mono" data-testid={`${svc}-key-hint`}>{hint}</span>
            {/if}
          </div>

          {#if reason}
            <p class="sync-reason-line sub">{reason}</p>
          {/if}

          {#if needsAutoConfirm && selectedCmdr}
            <div class="sync-callout callout-amber" data-testid={`${svc}-autoconfirm-banner`}>
              <p class="sub">{t('accounts-key-auto-confirm-banner', { cmdr: selectedCmdr.name })}</p>
              <div class="form-actions">
                <button
                  type="button"
                  class="action-btn"
                  onclick={() => confirmOwner(svc)}
                  data-testid={`${svc}-confirm-owner-btn`}
                >
                  {t('accounts-key-confirm-owner')}
                </button>
                {#if commanders.length > 1}
                  <div class="reassign-inline">
                    <select
                      class="glass-select"
                      bind:value={reassignTargets[svc]}
                      data-testid={`${svc}-reassign-select`}
                    >
                      {#each commanders as cmdr (cmdr.cmdr_ref)}
                        <option value={cmdr.cmdr_ref}>CMDR {cmdr.name}</option>
                      {/each}
                    </select>
                    <button
                      type="button"
                      class="action-btn btn-secondary"
                      onclick={() => assignKey(svc, reassignTargets[svc])}
                      data-testid={`${svc}-reassign-btn`}
                    >
                      {t('accounts-key-reassign-owner')}
                    </button>
                  </div>
                {/if}
              </div>
            </div>
          {/if}

          <div class="sync-metrics-grid">
            <div class="sync-metric-cell">
              <span class="metric-label">{t('accounts-sync-metric-sent')}</span>
              <span class="sync-metric-val mono cyan" data-testid={`${svc}-metric-sent`}>
                {integer(svcStatus?.sent_total ?? 0)}
              </span>
            </div>
            <div class="sync-metric-cell">
              <span class="metric-label">{t('accounts-sync-metric-pending')}</span>
              <span class="sync-metric-val mono" data-testid={`${svc}-metric-pending`}>
                {integer(svcStatus?.pending ?? 0)}
              </span>
            </div>
            <div class="sync-metric-cell">
              <span class="metric-label">{t('accounts-sync-metric-rejected')}</span>
              <span class="sync-metric-val mono" data-testid={`${svc}-metric-rejected`}>
                {integer(svcStatus?.rejected_total ?? 0)}
              </span>
            </div>
            <div class="sync-metric-cell">
              <span class="metric-label">{t('accounts-sync-metric-last-confirmed')}</span>
              <span class="sync-metric-val mono" data-testid={`${svc}-metric-last-confirmed`}>
                {formatLastConfirmed(svcStatus?.last_confirmed_at)}
              </span>
            </div>
          </div>

          <div class="input-form">
            <label class="sub mono" for={`${svc}-key`}>{t(`accounts-${svc}-api-key-label`)}</label>
            <div class="input-row">
              <input
                id={`${svc}-key`}
                class="glass-input mono"
                type={showKey[svc] ? 'text' : 'password'}
                placeholder={t(`accounts-${svc}-api-key-placeholder`)}
                bind:value={keyInputs[svc]}
                data-testid={`${svc}-key-input`}
              />
              <button
                class="icon-toggle-btn"
                type="button"
                aria-label={t('accounts-key-toggle-visibility')}
                onclick={() => (showKey[svc] = !showKey[svc])}
                data-testid={`${svc}-toggle-visibility-btn`}
              >
                {showKey[svc] ? '🙈' : '👁️'}
              </button>
            </div>
            <div class="form-actions">
              <button
                type="button"
                class="action-btn"
                onclick={() => saveKey(svc)}
                disabled={savingService === svc || !effectiveCmdrRef || !keyInputs[svc].trim()}
                data-testid={`${svc}-save-btn`}
              >
                {savingService === svc ? '...' : t('accounts-edsm-btn-save-verify')}
              </button>
              {#if hasKey}
                <button
                  type="button"
                  class="action-btn btn-secondary"
                  onclick={() => (pendingDeleteConfirm = svc)}
                  disabled={savingService === svc || !effectiveCmdrRef}
                  title={t('accounts-key-delete-confirm-note')}
                  data-testid={`${svc}-clear-btn`}
                >
                  {t('accounts-edsm-btn-delete')}
                </button>
              {/if}
            </div>

            {#if pendingDeleteConfirm === svc}
              <div class="confirm-banner" data-testid={`${svc}-delete-confirm-box`}>
                <p class="sub">{t('accounts-key-delete-confirm-prompt')}</p>
                <div class="form-actions">
                  <button
                    type="button"
                    class="action-btn btn-danger"
                    onclick={() => confirmDeleteKey(svc)}
                    data-testid={`${svc}-delete-confirm-btn`}
                  >
                    {t('accounts-ui-confirm')}
                  </button>
                  <button
                    type="button"
                    class="action-btn btn-secondary"
                    onclick={() => (pendingDeleteConfirm = null)}
                    data-testid={`${svc}-delete-cancel-btn`}
                  >
                    {t('accounts-ui-cancel')}
                  </button>
                </div>
              </div>
            {/if}

            {#if feedback[svc]}
              <p class="key-feedback feedback-{feedback[svc]!.tone}" data-testid={`${svc}-key-feedback`}>
                {feedback[svc]!.text}
              </p>
            {/if}
          </div>

          {#if svcStatus?.state === 'error'}
            <div class="card-foot">
              <button
                type="button"
                class="btn-sync"
                onclick={() => retryNow(svc)}
                disabled={busyAction === svc}
                data-testid={`${svc}-retry-btn`}
              >
                {busyAction === svc ? '...' : t('accounts-sync-retry-now')}
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .sync-keys-root {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
    min-width: 0;
  }

  .glass-card {
    background: var(--hud-surface, rgba(14, 22, 33, 0.95));
    border: 1px solid var(--glass-border, rgba(0, 212, 178, 0.2));
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .cmdr-selector-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 14px;
  }

  .cmdr-selector-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px 12px;
  }

  .cmdr-selector-controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    flex: 1 1 220px;
    justify-content: flex-end;
    min-width: 0;
  }

  .confirm-banner, .sync-callout {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 6px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.35);
  }

  .unassigned-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .unassigned-controls, .reassign-inline {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    min-width: 0;
    flex: 1 1 180px;
  }

  .keyed-services-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(290px, 1fr));
    gap: 12px;
    width: 100%;
    min-width: 0;
  }

  .hud-card {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }

  .service-identity {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 0;
  }

  .service-icon { font-size: 20px; line-height: 1.2; flex-shrink: 0; }
  .service-titles { min-width: 0; flex: 1; }
  .service-name { margin: 0; font-size: 14px; font-weight: 700; color: #f0f4f8; }
  .card-body { display: flex; flex-direction: column; gap: 10px; min-width: 0; }

  .status-row, .badges-wrap {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px 8px;
    min-width: 0;
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
  }

  .badge-green { background: rgba(16, 185, 129, 0.16); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.35); }
  .badge-amber { background: rgba(245, 158, 11, 0.16); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.35); }
  .badge-red { background: rgba(239, 68, 68, 0.16); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.35); }
  .badge-muted { background: rgba(148, 163, 184, 0.14); color: #cbd5e1; border: 1px solid rgba(148, 163, 184, 0.3); }

  .cmdr-pill {
    font-size: 11px;
    font-weight: 700;
    color: #00d4b2;
    background: rgba(0, 212, 178, 0.1);
    border: 1px solid rgba(0, 212, 178, 0.3);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .sync-metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
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
    color: var(--hud-muted, #94a3b8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sync-metric-val { font-size: 13px; font-weight: 700; color: #f0f4f8; }

  .input-form { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .input-row { display: flex; align-items: center; gap: 6px; min-width: 0; }

  .glass-input, .glass-select {
    flex: 1 1 160px;
    min-width: 0;
    background: rgba(10, 15, 25, 0.75);
    color: #f0f4f8;
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 5px;
    padding: 7px 10px;
    font-size: 12px;
  }

  .icon-toggle-btn {
    background: rgba(148, 163, 184, 0.12);
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 5px;
    padding: 6px 9px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .form-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    width: 100%;
  }

  .form-actions > .action-btn {
    flex: 1 1 120px;
  }

  .action-btn, .btn-outline, .btn-sync {
    border-radius: 5px;
    padding: 7px 11px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid rgba(56, 189, 248, 0.4);
    background: rgba(56, 189, 248, 0.16);
    color: #38bdf8;
  }

  .btn-secondary, .btn-outline {
    background: transparent;
    color: #cbd5e1;
    border-color: rgba(148, 163, 184, 0.35);
  }

  .btn-danger {
    background: rgba(239, 68, 68, 0.18);
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.4);
  }

  .btn-compact { padding: 5px 9px; font-size: 11px; }
  .key-feedback { margin: 0; font-size: 12px; line-height: 1.35; }
  .feedback-green { color: #34d399; }
  .feedback-amber { color: #fbbf24; }
  .feedback-red { color: #f87171; }
  .sub { margin: 0; font-size: 12px; color: var(--hud-muted, #94a3b8); line-height: 1.35; }
  .bold { font-weight: 700; }
  .mono { font-family: var(--font-mono, ui-monospace, monospace); }
  .cyan { color: var(--hud-info, #38bdf8); }
  .cmdr-empty-note { padding: 4px 2px; }
</style>
