<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, formatShipName, integer, ly, relativeAge } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import ManufacturerIcon from '../../lib/icons/ManufacturerIcon.svelte';
  import StationIcon from '../../lib/icons/StationIcon.svelte';
  import type {
    AccountsStatusTopic,
    RemoteShip,
    SyncReason,
    SyncServiceStatus,
    SyncState,
  } from './types';

  const status = $derived(topic<AccountsStatusTopic>('accounts.status'));

  let selectedCmdrRef = $state<string | null>(null);
  let edsmKey = $state('');
  let savingEdsm = $state(false);
  let edsmFeedback = $state<{ tone: 'green' | 'red' | 'amber'; text: string } | null>(null);
  let inaraKey = $state('');
  let savingInara = $state(false);
  let inaraFeedback = $state<{ tone: 'green' | 'red' | 'amber'; text: string } | null>(null);
  let showEdsmKey = $state(false);
  let showInaraKey = $state(false);
  let retryingService = $state<string | null>(null);
  let loadingFleet = $state(false);
  let fleetFetched = $state(false);
  let loginOpening = $state(false);
  let showManualCapi = $state(false);
  let manualCapiInput = $state('');
  let submittingManualCapi = $state(false);
  let manualCapiFeedback = $state<string | null>(null);
  let remoteShips = $state<RemoteShip[]>([]);
  let selectedShipId = $state<number | null>(null);
  const activeShip = $derived(remoteShips.find((s) => s.id === selectedShipId) ?? remoteShips[0]);

  function getShipLabels(ship: RemoteShip): { type: string; customName: string | null } {
    const hasValidType = ship.ship_type && ship.ship_type.toLowerCase() !== 'unknown';
    if (hasValidType) {
      return {
        type: formatShipName(ship.ship_type),
        customName: ship.name && ship.name !== ship.ship_type ? ship.name : null,
      };
    }
    // Fallback de contingencia: si ship_type era "Unknown", ship.name contiene el modelo reportado por CAPI
    if (ship.name) {
      return {
        type: formatShipName(ship.name),
        customName: null,
      };
    }
    return {
      type: t('ship-unknown-name'),
      customName: null,
    };
  }

  $effect(() => {
    if (status?.capi?.connected && !fleetFetched && !loadingFleet) {
      fleetFetched = true;
      fetchFleet();
    }
  });

  function stateIcon(state: SyncState | undefined): string {
    switch (state) {
      case 'connected':
        return '✓';
      case 'error':
        return '⚠';
      case 'waiting':
        return '⏳';
      case 'paused':
        return '⏸';
      case 'unavailable':
        return '⛔';
      default:
        return '○';
    }
  }

  function stateBadgeClass(state: SyncState | undefined): string {
    switch (state) {
      case 'connected':
        return 'badge-green';
      case 'error':
        return 'badge-red';
      case 'waiting':
      case 'paused':
        return 'badge-amber';
      default:
        return 'badge-muted';
    }
  }

  function reasonText(reason: SyncReason | null | undefined): string | null {
    if (!reason) return null;
    const kebab = reason.replaceAll('_', '-');
    return t(`accounts-sync-reason-${kebab}`);
  }

  function formatRetryTime(nextAttemptAt: string | null | undefined): string | null {
    if (!nextAttemptAt) return null;
    const targetMs = Date.parse(nextAttemptAt);
    if (Number.isNaN(targetMs)) return null;
    const secs = Math.max(1, Math.ceil((targetMs - Date.now()) / 1000));
    if (secs < 60) return `${secs} s`;
    if (secs < 3600) return `${Math.ceil(secs / 60)} min`;
    return `${Math.ceil(secs / 3600)} h`;
  }

  function formatStateLabel(svc: SyncServiceStatus | undefined): string {
    const state = svc?.state ?? 'disabled';
    if (state === 'error') {
      const retryTime = formatRetryTime(svc?.next_attempt_at);
      if (retryTime) {
        return t('accounts-sync-retry-in', { time: retryTime });
      }
    }
    return t(`accounts-sync-state-${state}`);
  }

  function formatLastConfirmed(iso: string | null | undefined): string {
    if (!iso) return t('accounts-sync-never');
    return relativeAge(iso, Date.now());
  }

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
  const selectedEdsmHint = $derived.by(() => {
    const raw = isSelectedCurrent || !selectedCmdr ? (status?.edsm?.key_hint ?? null) : null;
    if (!raw) return null;
    return raw.includes('•') ? raw : `••${raw}`;
  });
  const selectedEdsmHasKey = $derived(
    selectedCmdr ? selectedCmdr.edsm_key : Boolean(status?.edsm?.has_key)
  );
  const hasUnassignedEdsm = $derived((status?.unassigned_keys ?? []).includes('edsm'));
  const selectedInaraHint = $derived.by(() => {
    const raw = isSelectedCurrent || !selectedCmdr ? (status?.inara?.key_hint ?? null) : null;
    if (!raw) return null;
    return raw.includes('•') ? raw : `••${raw}`;
  });
  const selectedInaraHasKey = $derived(
    selectedCmdr ? selectedCmdr.inara_key : Boolean(status?.inara?.has_key)
  );
  const hasUnassignedInara = $derived((status?.unassigned_keys ?? []).includes('inara'));
  const needsAutoConfirm = $derived(selectedCmdr?.key_assignment === 'auto_pending_confirmation');

  $effect(() => {
    if (commanders.length > 0 && (!selectedCmdrRef || !commanders.some((c) => c.cmdr_ref === selectedCmdrRef))) {
      selectedCmdrRef = status?.current_cmdr?.cmdr_ref ?? commanders[0]?.cmdr_ref ?? null;
    }
  });

  async function toggleEddn() {
    await action('accounts.eddn.set_enabled', { enabled: !status?.eddn?.enabled });
  }

  async function saveEdsmKey() {
    const cmdrRef = effectiveCmdrRef;
    if (!edsmKey.trim() || !cmdrRef) return;
    savingEdsm = true;
    edsmFeedback = null;
    try {
      const res = (await action('accounts.key.save', {
        service: 'edsm',
        cmdr_ref: cmdrRef,
        api_key: edsmKey.trim(),
      })) as { ok?: boolean; verified?: boolean; state?: string } | undefined;
      if (res?.state === 'rejected') {
        edsmFeedback = { tone: 'red', text: t('accounts-key-verify-rejected') };
      } else if (res?.verified || res?.state === 'active') {
        edsmKey = '';
        edsmFeedback = { tone: 'green', text: t('accounts-key-verify-active') };
      } else {
        edsmKey = '';
        edsmFeedback = { tone: 'amber', text: t('accounts-key-verify-unverified') };
      }
    } finally {
      savingEdsm = false;
    }
  }

  async function deleteEdsmKey() {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    savingEdsm = true;
    edsmFeedback = null;
    try {
      const res = (await action('accounts.key.delete', {
        service: 'edsm',
        cmdr_ref: cmdrRef,
      })) as { discarded?: number } | undefined;
      edsmFeedback = {
        tone: 'amber',
        text: t('accounts-key-pending-discarded', { count: res?.discarded ?? 0 }),
      };
    } finally {
      savingEdsm = false;
    }
  }

  async function confirmKeyOwner(service: 'edsm' | 'inara') {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    await action('accounts.key.confirm', { service, cmdr_ref: cmdrRef });
  }

  async function assignUnassignedKey(service: 'edsm' | 'inara') {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    await action('accounts.key.assign', { service, cmdr_ref: cmdrRef });
  }

  async function forgetCommander() {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    await action('accounts.cmdr.forget', { cmdr_ref: cmdrRef });
    if (selectedCmdrRef === cmdrRef) {
      selectedCmdrRef = null;
    }
  }

  async function saveInaraKey() {
    const cmdrRef = effectiveCmdrRef;
    if (!inaraKey.trim() || !cmdrRef) return;
    savingInara = true;
    inaraFeedback = null;
    try {
      const res = (await action('accounts.key.save', {
        service: 'inara',
        cmdr_ref: cmdrRef,
        api_key: inaraKey.trim(),
      })) as { ok?: boolean; verified?: boolean; state?: string } | undefined;
      if (res?.state === 'rejected') {
        inaraFeedback = { tone: 'red', text: t('accounts-key-verify-rejected') };
      } else if (res?.verified || res?.state === 'active') {
        inaraKey = '';
        inaraFeedback = { tone: 'green', text: t('accounts-key-verify-active') };
      } else {
        inaraKey = '';
        inaraFeedback = { tone: 'amber', text: t('accounts-key-verify-unverified') };
      }
    } finally {
      savingInara = false;
    }
  }

  async function deleteInaraKey() {
    const cmdrRef = effectiveCmdrRef;
    if (!cmdrRef) return;
    savingInara = true;
    inaraFeedback = null;
    try {
      const res = (await action('accounts.key.delete', {
        service: 'inara',
        cmdr_ref: cmdrRef,
      })) as { discarded?: number } | undefined;
      inaraFeedback = {
        tone: 'amber',
        text: t('accounts-key-pending-discarded', { count: res?.discarded ?? 0 }),
      };
    } finally {
      savingInara = false;
    }
  }

  async function retryNow(serviceId: string) {
    retryingService = serviceId;
    try {
      await action('accounts.sync_now');
    } finally {
      retryingService = null;
    }
  }

  const officialRedirectUri = 'https://danikuso.github.io/TheMultipurposeCompanion/oauth/callback.html';

  async function startFrontierLogin() {
    loginOpening = true;
    try {
      const res = await fetch(
        `/api/auth/frontier/login?open=true&redirect_uri=${encodeURIComponent(officialRedirectUri)}`,
        {
          headers: { Accept: 'application/json' },
        }
      );
      if (res.ok) {
        const data = await res.json();
        if (!data.opened && data.url && typeof window !== 'undefined') {
          window.open(data.url, '_blank');
        }
      } else if (typeof window !== 'undefined') {
        window.open(
          `/api/auth/frontier/login?redirect_uri=${encodeURIComponent(officialRedirectUri)}`,
          '_blank'
        );
      }
    } catch {
      if (typeof window !== 'undefined') {
        window.open('/api/auth/frontier/login', '_blank');
      }
    } finally {
      setTimeout(() => {
        loginOpening = false;
      }, 2000);
    }
  }

  async function unlinkFrontier() {
    await action('frontier.logout');
    remoteShips = [];
    fleetFetched = false;
  }

  async function submitManualCapi() {
    if (!manualCapiInput.trim()) return;
    submittingManualCapi = true;
    manualCapiFeedback = null;
    try {
      const res = (await action('frontier.submit_callback', {
        url: manualCapiInput.trim(),
      })) as { ok?: boolean };
      if (res?.ok) {
        manualCapiFeedback = t('accounts-capi-manual-success');
        manualCapiInput = '';
        fetchFleet();
        setTimeout(() => {
          showManualCapi = false;
          manualCapiFeedback = null;
        }, 2000);
      } else {
        manualCapiFeedback = t('accounts-capi-manual-failed');
      }
    } catch (e: any) {
      manualCapiFeedback = `${t('accounts-capi-manual-failed')} (${e?.message ?? 'error'})`;
    } finally {
      submittingManualCapi = false;
    }
  }

  async function fetchFleet() {
    loadingFleet = true;
    try {
      const res = (await action('frontier.fleet')) as { ships?: RemoteShip[] };
      if (res?.ships) {
        remoteShips = res.ships;
      }
    } catch {
      // Ignorar error si no está vinculada
    } finally {
      loadingFleet = false;
    }
  }

  const currentCmdrName = $derived(status?.current_cmdr?.name ?? null);
</script>

<div class="accounts-container">
  <!-- Banner de Política de Privacidad Innegociable -->
  <div class="privacy-banner glass-card" data-testid="accounts-privacy-banner">
    <div class="shield-icon">🛡️</div>
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

  {#if commanders.length > 0}
    <div class="cmdr-selector-bar glass-card" data-testid="accounts-cmdr-bar">
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
            edsmFeedback = null;
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
            onclick={forgetCommander}
            data-testid="accounts-cmdr-forget-btn"
          >
            {t('accounts-cmdr-forget')}
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <div class="accounts-grid">
    <!-- 1. TARJETA EDDN -->
    <div class="hud-card glass-card" data-testid="accounts-eddn-card">
      <div class="card-head">
        <div class="service-identity">
          <span class="service-icon">📡</span>
          <div class="service-titles">
            <h4 class="service-name">{t('accounts-eddn-title')}</h4>
            <p class="sub">{t('accounts-eddn-desc')}</p>
          </div>
        </div>
        <button
          class="switch-btn"
          class:active={status?.eddn?.enabled}
          onclick={toggleEddn}
          data-testid="eddn-switch"
          aria-label={t('accounts-eddn-switch')}
        >
          <span class="switch-knob"></span>
        </button>
      </div>

      <div class="card-body">
        <div class="status-row">
          <span
            class="badge {stateBadgeClass(status?.eddn?.state)}"
            data-testid="eddn-state-pill"
            title={reasonText(status?.eddn?.reason) ?? formatStateLabel(status?.eddn)}
          >
            {stateIcon(status?.eddn?.state)} {formatStateLabel(status?.eddn)}
          </span>
        </div>
        {#if reasonText(status?.eddn?.reason)}
          <p class="sync-reason-line sub">{reasonText(status?.eddn?.reason)}</p>
        {/if}
        {#if (status?.eddn?.paused_schemas?.length ?? 0) > 0}
          <p class="sync-reason-line sub mono" data-testid="eddn-paused-schemas">
            {t('accounts-eddn-paused-schemas', { schemas: status?.eddn?.paused_schemas?.join(', ') ?? '' })}
          </p>
        {/if}

        <div class="sync-metrics-grid">
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-sent')}</span>
            <span class="sync-metric-val mono cyan" data-testid="eddn-metric-sent">
              {integer(status?.eddn?.sent_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-pending')}</span>
            <span class="sync-metric-val mono" data-testid="eddn-metric-pending">
              {integer(status?.eddn?.pending ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-rejected')}</span>
            <span class="sync-metric-val mono" data-testid="eddn-metric-rejected">
              {integer(status?.eddn?.rejected_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-last-confirmed')}</span>
            <span class="sync-metric-val mono" data-testid="eddn-metric-last-confirmed">
              {formatLastConfirmed(status?.eddn?.last_confirmed_at)}
            </span>
          </div>
        </div>

        {#if status?.eddn?.state === 'error'}
          <div class="card-foot">
            <button
              class="btn-sync"
              onclick={() => retryNow('eddn')}
              disabled={retryingService === 'eddn'}
              data-testid="eddn-retry-btn"
            >
              {retryingService === 'eddn' ? '...' : t('accounts-sync-retry-now')}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- 2. TARJETA EDSM -->
    <div class="hud-card glass-card" data-testid="accounts-edsm-card">
      <div class="card-head">
        <div class="service-identity">
          <span class="service-icon">🌌</span>
          <div class="service-titles">
            <h4 class="service-name">{t('accounts-edsm-title')}</h4>
            <p class="sub">{t('accounts-edsm-desc')}</p>
          </div>
        </div>
      </div>

      <div class="card-body">
        <div class="status-row">
          <span
            class="badge {stateBadgeClass(status?.edsm?.state)}"
            data-testid="edsm-state-pill"
            title={reasonText(status?.edsm?.reason) ?? formatStateLabel(status?.edsm)}
          >
            {stateIcon(status?.edsm?.state)} {formatStateLabel(status?.edsm)}
          </span>
          {#if selectedEdsmHint}
            <span class="cmdr-pill mono" data-testid="edsm-key-hint">{selectedEdsmHint}</span>
          {/if}
        </div>
        {#if reasonText(status?.edsm?.reason)}
          <p class="sync-reason-line sub">{reasonText(status?.edsm?.reason)}</p>
        {/if}
        {#if hasUnassignedEdsm && effectiveCmdrRef}
          <div class="sync-callout callout-amber" data-testid="edsm-unassigned-banner">
            <p class="sub">{t('accounts-key-unassigned-banner')}</p>
            <button
              type="button"
              class="btn-sync btn-compact"
              onclick={() => assignUnassignedKey('edsm')}
              data-testid="edsm-assign-key-btn"
            >
              {t('accounts-key-unassigned-assign')}
            </button>
          </div>
        {/if}
        {#if needsAutoConfirm && selectedEdsmHasKey && selectedCmdr}
          <div class="sync-callout callout-amber" data-testid="edsm-autoconfirm-banner">
            <p class="sub">{t('accounts-key-auto-confirm-banner', { cmdr: selectedCmdr.name })}</p>
            <button
              type="button"
              class="btn-sync btn-compact"
              onclick={() => confirmKeyOwner('edsm')}
              data-testid="edsm-confirm-owner-btn"
            >
              {t('accounts-key-confirm-owner')}
            </button>
          </div>
        {/if}

        <div class="sync-metrics-grid">
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-sent')}</span>
            <span class="sync-metric-val mono cyan" data-testid="edsm-metric-sent">
              {integer(status?.edsm?.sent_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-pending')}</span>
            <span class="sync-metric-val mono" data-testid="edsm-metric-pending">
              {integer(status?.edsm?.pending ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-rejected')}</span>
            <span class="sync-metric-val mono" data-testid="edsm-metric-rejected">
              {integer(status?.edsm?.rejected_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-last-confirmed')}</span>
            <span class="sync-metric-val mono" data-testid="edsm-metric-last-confirmed">
              {formatLastConfirmed(status?.edsm?.last_confirmed_at)}
            </span>
          </div>
        </div>

        <div class="input-form">
          <label class="sub mono" for="edsm-key">{t('accounts-edsm-api-key-label')}</label>
          <div class="input-row">
            <input
              id="edsm-key"
              class="glass-input mono"
              type={showEdsmKey ? 'text' : 'password'}
              placeholder={t('accounts-edsm-api-key-placeholder')}
              bind:value={edsmKey}
              data-testid="edsm-key-input"
            />
            <button class="icon-toggle-btn" onclick={() => (showEdsmKey = !showEdsmKey)} type="button">
              {showEdsmKey ? '🙈' : '👁️'}
            </button>
          </div>
          <div class="btn-group-row">
            <button
              class="action-btn flex-1"
              onclick={saveEdsmKey}
              disabled={savingEdsm || !effectiveCmdrRef || !edsmKey.trim()}
              data-testid="edsm-save-btn"
            >
              {savingEdsm ? '...' : t('accounts-edsm-btn-save-verify')}
            </button>
            {#if selectedEdsmHasKey}
              <button
                class="action-btn btn-secondary flex-1"
                onclick={deleteEdsmKey}
                disabled={savingEdsm || !effectiveCmdrRef}
                title={t('accounts-key-delete-confirm-note')}
                data-testid="edsm-clear-btn"
              >
                {t('accounts-edsm-btn-delete')}
              </button>
            {/if}
          </div>
          {#if edsmFeedback}
            <p class="key-feedback feedback-{edsmFeedback.tone}" data-testid="edsm-key-feedback">
              {edsmFeedback.text}
            </p>
          {/if}
        </div>

        {#if status?.edsm?.state === 'error'}
          <div class="card-foot">
            <button
              class="btn-sync"
              onclick={() => retryNow('edsm')}
              disabled={retryingService === 'edsm'}
              data-testid="edsm-retry-btn"
            >
              {retryingService === 'edsm' ? '...' : t('accounts-sync-retry-now')}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- 3. TARJETA INARA -->
    <div class="hud-card glass-card" data-testid="accounts-inara-card">
      <div class="card-head">
        <div class="service-identity">
          <span class="service-icon">🎖️</span>
          <div class="service-titles">
            <h4 class="service-name">{t('accounts-inara-title')}</h4>
            <p class="sub">{t('accounts-inara-desc')}</p>
          </div>
        </div>
      </div>

      <div class="card-body">
        <div class="status-row">
          <div class="badges-wrap">
            <span
              class="badge {stateBadgeClass(status?.inara?.state)}"
              data-testid="inara-state-pill"
              title={reasonText(status?.inara?.reason) ?? formatStateLabel(status?.inara)}
            >
              {stateIcon(status?.inara?.state)} {formatStateLabel(status?.inara)}
            </span>
            <span class="badge badge-green" data-testid="inara-whitelist-badge" title={t('accounts-inara-whitelist-tooltip')}>
              🛡️ {t('accounts-inara-whitelist-badge')}
            </span>
          </div>
          {#if selectedInaraHint}
            <span class="cmdr-pill mono" data-testid="inara-key-hint">{selectedInaraHint}</span>
          {/if}
        </div>
        {#if reasonText(status?.inara?.reason)}
          <p class="sync-reason-line sub">{reasonText(status?.inara?.reason)}</p>
        {/if}
        {#if hasUnassignedInara && effectiveCmdrRef}
          <div class="sync-callout callout-amber" data-testid="inara-unassigned-banner">
            <p class="sub">{t('accounts-key-unassigned-banner')}</p>
            <button
              type="button"
              class="btn-sync btn-compact"
              onclick={() => assignUnassignedKey('inara')}
              data-testid="inara-assign-key-btn"
            >
              {t('accounts-key-unassigned-assign')}
            </button>
          </div>
        {/if}
        {#if needsAutoConfirm && selectedInaraHasKey && selectedCmdr}
          <div class="sync-callout callout-amber" data-testid="inara-autoconfirm-banner">
            <p class="sub">{t('accounts-key-auto-confirm-banner', { cmdr: selectedCmdr.name })}</p>
            <button
              type="button"
              class="btn-sync btn-compact"
              onclick={() => confirmKeyOwner('inara')}
              data-testid="inara-confirm-owner-btn"
            >
              {t('accounts-key-confirm-owner')}
            </button>
          </div>
        {/if}

        <div class="sync-metrics-grid">
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-sent')}</span>
            <span class="sync-metric-val mono cyan" data-testid="inara-metric-sent">
              {integer(status?.inara?.sent_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-pending')}</span>
            <span class="sync-metric-val mono" data-testid="inara-metric-pending">
              {integer(status?.inara?.pending ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-rejected')}</span>
            <span class="sync-metric-val mono" data-testid="inara-metric-rejected">
              {integer(status?.inara?.rejected_total ?? 0)}
            </span>
          </div>
          <div class="sync-metric-cell">
            <span class="metric-label">{t('accounts-sync-metric-last-confirmed')}</span>
            <span class="sync-metric-val mono" data-testid="inara-metric-last-confirmed">
              {formatLastConfirmed(status?.inara?.last_confirmed_at)}
            </span>
          </div>
        </div>

        <div class="input-form">
          <label class="sub mono" for="inara-key">{t('accounts-inara-api-key-label')}</label>
          <div class="input-row">
            <input
              id="inara-key"
              class="glass-input mono"
              type={showInaraKey ? 'text' : 'password'}
              placeholder={t('accounts-inara-api-key-placeholder')}
              bind:value={inaraKey}
              data-testid="inara-key-input"
            />
            <button class="icon-toggle-btn" onclick={() => (showInaraKey = !showInaraKey)} type="button">
              {showInaraKey ? '🙈' : '👁️'}
            </button>
          </div>
          <div class="btn-group-row">
            <button
              class="action-btn flex-1"
              onclick={saveInaraKey}
              disabled={savingInara || !effectiveCmdrRef || !inaraKey.trim()}
              data-testid="inara-save-btn"
            >
              {savingInara ? '...' : t('accounts-edsm-btn-save-verify')}
            </button>
            {#if selectedInaraHasKey}
              <button
                class="action-btn btn-secondary flex-1"
                onclick={deleteInaraKey}
                disabled={savingInara || !effectiveCmdrRef}
                title={t('accounts-key-delete-confirm-note')}
                data-testid="inara-clear-btn"
              >
                {t('accounts-edsm-btn-delete')}
              </button>
            {/if}
          </div>
          {#if inaraFeedback}
            <p class="key-feedback feedback-{inaraFeedback.tone}" data-testid="inara-key-feedback">
              {inaraFeedback.text}
            </p>
          {/if}
        </div>

        {#if status?.inara?.state === 'error'}
          <div class="card-foot">
            <button
              class="btn-sync"
              onclick={() => retryNow('inara')}
              disabled={retryingService === 'inara'}
              data-testid="inara-retry-btn"
            >
              {retryingService === 'inara' ? '...' : t('accounts-sync-retry-now')}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- 4. TARJETA FRONTIER CAPI (EXPANDIDA A TODA LA FILA) -->
    <div class="hud-card glass-card capi-card" data-testid="accounts-capi-card">
      <div class="card-head capi-card-head">
        <div class="service-identity">
          <span class="service-icon">🚀</span>
          <div class="service-titles">
            <div class="capi-title-row">
              <h4 class="service-name">{t('accounts-capi-title')}</h4>
              <span class="badge" class:badge-green={status?.capi?.connected} class:badge-amber={!status?.capi?.connected}>
                {status?.capi?.connected ? t('accounts-capi-status-linked') : t('accounts-capi-status-unlinked')}
              </span>
              {#if status?.capi?.connected && status?.capi?.commander_name}
                <span class="cmdr-pill mono">CMDR {status.capi.commander_name}</span>
              {/if}
            </div>
            <p class="sub">{t('accounts-capi-desc')}</p>
          </div>
        </div>
        {#if status?.capi?.connected}
          <div class="btn-group capi-actions">
            <button class="btn-outline" onclick={fetchFleet} disabled={loadingFleet} data-testid="capi-refresh-fleet-btn">
              {loadingFleet ? '...' : t('accounts-capi-btn-refresh-fleet')}
            </button>
            <button class="btn-danger" onclick={unlinkFrontier} data-testid="capi-unlink-btn">
              {t('accounts-capi-btn-unlink')}
            </button>
          </div>
        {/if}
      </div>

      <div class="card-body">
        {#if !status?.capi?.connected}
          <div class="link-prompt">
            <p class="sub">{t('accounts-capi-auth-notice')}</p>
            <div class="btn-group-row">
              <button class="btn-primary" onclick={startFrontierLogin} disabled={loginOpening} data-testid="capi-link-btn">
                {loginOpening ? t('accounts-capi-btn-opening') : t('accounts-capi-btn-link')}
              </button>
            </div>

            <div class="capi-manual-row">
              <button
                type="button"
                class="manual-toggle-btn sub mono"
                onclick={() => (showManualCapi = !showManualCapi)}
                data-testid="capi-manual-toggle"
              >
                {showManualCapi ? `▲ ${t('accounts-capi-manual-toggle-hide')}` : `▼ ${t('accounts-capi-manual-toggle')}`}
              </button>
              {#if showManualCapi}
                <div class="manual-input-box">
                  <p class="manual-help sub">{t('accounts-capi-manual-help')}</p>
                  <div class="input-row">
                    <input
                      class="glass-input mono"
                      type="text"
                      placeholder={t('accounts-capi-code-placeholder')}
                      bind:value={manualCapiInput}
                      data-testid="capi-manual-input"
                    />
                    <button
                      class="action-btn"
                      onclick={submitManualCapi}
                      disabled={submittingManualCapi || !manualCapiInput.trim()}
                      data-testid="capi-manual-submit"
                    >
                      {submittingManualCapi ? '...' : t('accounts-capi-btn-complete')}
                    </button>
                  </div>
                  {#if manualCapiFeedback}
                    <p class="test-feedback mono" data-testid="capi-manual-feedback">{manualCapiFeedback}</p>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <!-- Flota remota -->
          <div class="fleet-container" data-testid="capi-remote-fleet-list">
            {#if remoteShips.length === 0}
              <div class="capi-empty-state">
                <div class="capi-stats-bar">
                  <div class="stat-item">
                    <span class="sub">{t('accounts-capi-remote-fleet-count')}</span>
                    <span class="mono cyan bold stat-val">{status?.capi?.remote_ships_count ?? 0}</span>
                  </div>
                  {#if status?.capi?.last_sync_time}
                    <div class="stat-item">
                      <span class="sub">{t('accounts-edsm-last-sync')}</span>
                      <span class="mono stat-val">{status.capi.last_sync_time}</span>
                    </div>
                  {/if}
                </div>
                <div class="capi-info-box">
                  <span class="info-icon">ℹ️</span>
                  <div class="info-text">
                    <p class="info-empty-title">{t('accounts-capi-remote-fleet-empty')}</p>
                    <p class="info-empty-desc">{t('accounts-capi-empty-hint')}</p>
                  </div>
                </div>
              </div>
            {:else}
              <div class="fleet-cockpit-layout">
                <!-- Columna izquierda: Nave seleccionada -->
                <div class="fleet-detail-col">
                  <label class="compact-selector-label" for="remote-fleet-select">
                    <span class="label-text">{t('ui-count-heading', { label: t('field-ship'), count: remoteShips.length })}</span>
                    <select
                      id="remote-fleet-select"
                      class="glass-select"
                      data-testid="remote-fleet-select"
                      value={selectedShipId ?? activeShip?.id ?? ''}
                      onchange={(e) => (selectedShipId = Number(e.currentTarget.value))}
                    >
                      {#each remoteShips as ship (ship.id)}
                        {@const labels = getShipLabels(ship)}
                        <option value={ship.id}>
                          {labels.type}{labels.customName ? ` «${labels.customName}»` : ''} - {ship.system}
                        </option>
                      {/each}
                    </select>
                  </label>

                  {#if activeShip}
                    {@const activeLabels = getShipLabels(activeShip)}
                    <div class="active-ship-card">
                      <div class="ship-card-head">
                        <div class="ship-top">
                          <span class="ship-type mono bold">{activeLabels.type}</span>
                          {#if activeLabels.customName}
                            <span class="ship-custom-name">«{activeLabels.customName}»</span>
                          {/if}
                        </div>
                        <div class="ship-mfr" title={activeLabels.type}>
                          <ManufacturerIcon shipType={activeLabels.type} size={24} />
                        </div>
                      </div>
                      <div class="ship-loc sub">
                        <StationIcon size={14} />
                        <span>{activeShip.station} ({activeShip.system})</span>
                      </div>
                      <div class="ship-meta sub mono">
                        <span class="gold">{credits(activeShip.value)}</span>
                        {#if activeShip.distance_ly != null}
                          <span>{ly(activeShip.distance_ly)}</span>
                        {/if}
                      </div>
                    </div>
                  {/if}
                </div>

                <!-- Columna derecha: Grid de naves almacenadas -->
                <div class="fleet-grid-col">
                  <div class="fleet-grid-head">
                    <span class="sub bold">{t('accounts-capi-remote-fleet-title')} ({remoteShips.length})</span>
                    <span class="sub mono cyan">{t('accounts-select-ship-hint')}</span>
                  </div>
                  <div class="fleet-cards-grid">
                    {#each remoteShips as ship (ship.id)}
                      {@const labels = getShipLabels(ship)}
                      <button
                        type="button"
                        class="ship-card interactive-ship-card"
                        class:is-selected={ship.id === activeShip?.id}
                        onclick={() => (selectedShipId = ship.id)}
                      >
                        <div class="ship-card-head">
                          <div class="ship-top">
                            <span class="ship-type mono bold">{labels.type}</span>
                            {#if labels.customName}
                              <span class="ship-custom-name">«{labels.customName}»</span>
                            {/if}
                          </div>
                          <div class="ship-mfr" title={labels.type}>
                            <ManufacturerIcon shipType={labels.type} size={18} />
                          </div>
                        </div>
                        <div class="ship-loc sub">
                          <StationIcon size={13} />
                          <span>{ship.station}</span>
                        </div>
                        <div class="ship-meta sub mono">
                          <span class="gold">{credits(ship.value)}</span>
                          <span>{ship.system}</span>
                        </div>
                      </button>
                    {/each}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .accounts-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 0;
    width: 100%;
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
    gap: 16px;
    padding: 14px 20px;
    border-left: 3px solid #00d4b2;
    background: rgba(0, 212, 178, 0.05);
  }

  .shield-icon {
    font-size: 28px;
    line-height: 1;
    flex-shrink: 0;
  }

  .privacy-text {
    font-size: 13px;
    color: var(--text-dim, #8fa3b8);
    margin-top: 4px;
    line-height: 1.4;
  }

  .cmdr-selector-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 16px;
    padding: 10px 16px;
  }

  .cmdr-selector-label {
    flex-shrink: 0;
  }

  .cmdr-selector-controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    flex: 1;
    min-width: min(220px, 100%);
    max-width: 460px;
  }

  .btn-compact {
    padding: 5px 10px;
    font-size: 11px;
    flex-shrink: 0;
  }

  .sync-callout {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 8px 10px;
    border-radius: 6px;
    margin-bottom: 10px;
  }

  .sync-callout p {
    margin: 0;
    line-height: 1.35;
  }

  .callout-amber {
    background: rgba(229, 169, 59, 0.1);
    border: 1px solid rgba(229, 169, 59, 0.35);
  }

  .key-feedback {
    font-size: 11px;
    margin: 4px 0 0 0;
    line-height: 1.35;
  }

  .feedback-green {
    color: #00d4b2;
  }

  .feedback-red {
    color: #ff7b5c;
  }

  .feedback-amber {
    color: #e5a93b;
  }

  .accounts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(300px, 100%), 1fr));
    gap: 16px;
  }

  .hud-card {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .service-identity {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
  }

  .service-titles {
    flex: 1;
    min-width: 0;
  }

  .service-icon {
    font-size: 24px;
    line-height: 1;
    flex-shrink: 0;
  }

  .service-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-main, #e6f1ff);
    margin: 0;
  }

  .sub {
    font-size: 12px;
    color: var(--text-dim, #7e9bb5);
  }

  .bold {
    font-weight: 600;
  }

  .mono {
    font-family: var(--mono-font, ui-monospace, monospace);
  }

  .cyan {
    color: #00d4b2;
  }

  .gold {
    color: #e5a93b;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .badge-green {
    background: rgba(0, 212, 178, 0.15);
    color: #00d4b2;
    border: 1px solid rgba(0, 212, 178, 0.4);
  }

  .badge-amber {
    background: rgba(229, 169, 59, 0.15);
    color: #e5a93b;
    border: 1px solid rgba(229, 169, 59, 0.4);
  }

  .badge-red {
    background: rgba(255, 102, 68, 0.15);
    color: #ff7b5c;
    border: 1px solid rgba(255, 102, 68, 0.45);
  }

  .badge-muted {
    background: rgba(148, 163, 184, 0.14);
    color: #94a3b8;
    border: 1px solid rgba(148, 163, 184, 0.35);
  }

  .cmdr-pill {
    font-size: 11px;
    background: rgba(255, 255, 255, 0.06);
    padding: 2px 8px;
    border-radius: 4px;
    color: #cbd5e1;
  }

  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 6px;
  }

  .sync-reason-line {
    margin: 0 0 10px 0;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .badges-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .sync-metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 12px;
  }

  .sync-metric-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 9px;
    background: rgba(0, 0, 0, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 5px;
    min-width: 0;
  }

  .metric-label {
    font-size: 11px;
    color: var(--text-dim, #7e9bb5);
  }

  .sync-metric-val {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-main, #e6f1ff);
    overflow-wrap: anywhere;
  }

  /* Switches */
  .switch-btn {
    width: 38px;
    min-width: 38px;
    max-width: 38px;
    height: 20px;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.25);
    border-radius: 10px;
    cursor: pointer;
    position: relative;
    padding: 0;
    flex-shrink: 0;
    transition: all 150ms ease;
    box-sizing: border-box;
  }

  .switch-btn:hover {
    border-color: rgba(255, 255, 255, 0.45);
  }

  .switch-btn.active {
    background: var(--hud-cyan, #00d4b2);
    border-color: var(--hud-cyan, #00d4b2);
    box-shadow: 0 0 8px rgba(0, 212, 178, 0.35);
  }

  .switch-knob {
    width: 14px;
    height: 14px;
    background: #ffffff;
    border-radius: 50%;
    position: absolute;
    top: 2px;
    left: 2px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    transition: transform 150ms ease;
  }

  .switch-btn.active .switch-knob {
    transform: translateX(18px);
  }

  /* Form inputs */
  .input-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 10px;
  }

  .input-row {
    display: flex;
    gap: 6px;
    min-width: 0;
  }

  .btn-group-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    width: 100%;
    margin-top: 2px;
  }

  .flex-1 {
    flex: 1;
    min-width: 120px;
    text-align: center;
  }

  .glass-input {
    flex: 1;
    min-width: 0;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    padding: 6px 10px;
    color: #fff;
    font-size: 12px;
  }

  .glass-input:focus {
    outline: none;
    border-color: #00d4b2;
  }

  .icon-toggle-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    padding: 0 8px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .action-btn,
  .btn-sync,
  .btn-primary,
  .btn-outline,
  .btn-danger {
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    white-space: nowrap;
  }

  .action-btn,
  .btn-sync {
    background: rgba(0, 212, 178, 0.15);
    color: #00d4b2;
    border: 1px solid rgba(0, 212, 178, 0.4);
  }

  .btn-primary {
    background: #00d4b2;
    color: #080c10;
    border: none;
  }

  .btn-outline {
    background: transparent;
    color: #cbd5e1;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.08);
    color: #e2e8f0;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .test-feedback {
    font-size: 11px;
    color: #00e5ff;
    margin: 4px 0 0 0;
  }

  .link-prompt {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .capi-manual-row {
    margin-top: 6px;
    width: 100%;
    text-align: left;
  }

  .manual-toggle-btn {
    background: transparent;
    border: none;
    color: var(--hud-cyan, #00d4b2);
    font-size: 11px;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    opacity: 0.85;
    transition: opacity 150ms ease;
  }

  .manual-toggle-btn:hover {
    opacity: 1;
  }

  .manual-input-box {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: rgba(0, 0, 0, 0.2);
    padding: 8px 10px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .manual-help {
    font-size: 11px;
    color: var(--text-dim, #7e9bb5);
    line-height: 1.3;
    margin: 0;
  }

  .btn-danger {
    background: rgba(255, 102, 68, 0.15);
    color: #ff6644;
    border: 1px solid rgba(255, 102, 68, 0.4);
  }

  .card-foot {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* CAPI Card & Full-Width layout */
  .capi-card {
    grid-column: 1 / -1;
  }

  .capi-card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .capi-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 2px;
  }

  .capi-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .capi-empty-state {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 6px 0;
  }

  .capi-stats-bar {
    display: flex;
    gap: 24px;
    align-items: center;
    flex-wrap: wrap;
  }

  .stat-item {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .stat-val {
    font-size: 15px;
  }

  .capi-info-box {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    padding: 12px 16px;
    background: rgba(0, 212, 178, 0.05);
    border: 1px solid rgba(0, 212, 178, 0.2);
    border-radius: 6px;
  }

  .info-icon {
    font-size: 20px;
    line-height: 1.2;
    flex-shrink: 0;
  }

  .info-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .info-empty-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-main, #e6f1ff);
    margin: 0;
  }

  .info-empty-desc {
    font-size: 12px;
    color: var(--text-dim, #8fa3b8);
    line-height: 1.45;
    margin: 0;
  }

  /* Cockpit Fleet Layout (2 columns) */
  .fleet-cockpit-layout {
    display: grid;
    grid-template-columns: minmax(min(280px, 100%), 320px) 1fr;
    gap: 16px;
    align-items: start;
  }

  @media (max-width: 860px) {
    .fleet-cockpit-layout {
      grid-template-columns: 1fr;
    }
  }

  .fleet-detail-col {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .fleet-grid-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .fleet-grid-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .fleet-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(200px, 100%), 1fr));
    gap: 10px;
    max-height: 280px;
    overflow-y: auto;
    padding-right: 4px;
    scrollbar-width: thin;
    scrollbar-color: rgba(0, 212, 178, 0.45) rgba(0, 0, 0, 0.2);
  }

  .compact-selector-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--hud-muted, #7ea0b8);
    width: 100%;
    flex-wrap: wrap;
  }

  .compact-selector-label .label-text {
    white-space: nowrap;
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.05em;
  }

  .glass-select {
    flex: 1;
    min-width: 0;
    background: rgba(10, 16, 26, 0.9);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.3));
    color: var(--hud-primary, #00e5ff);
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 12px;
    outline: none;
    cursor: pointer;
  }

  .active-ship-card {
    background: rgba(14, 22, 38, 0.8);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.3));
    border-left: 3px solid var(--hud-primary, #00e5ff);
    border-radius: 6px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ship-card {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: inherit;
  }

  .interactive-ship-card {
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
    width: 100%;
    box-sizing: border-box;
  }

  .interactive-ship-card:hover {
    background: rgba(0, 212, 178, 0.08);
    border-color: rgba(0, 212, 178, 0.4);
    transform: translateY(-1px);
  }

  .interactive-ship-card.is-selected {
    background: rgba(0, 212, 178, 0.12);
    border-color: #00d4b2;
    box-shadow: 0 0 8px rgba(0, 212, 178, 0.25);
  }

  .ship-card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .ship-top,
  .ship-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .ship-mfr {
    opacity: 0.85;
    display: flex;
    align-items: center;
  }

  .ship-loc {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-dim, #8fa3b8);
  }

  .ship-type {
    color: #fff;
    font-size: 13px;
  }

  .ship-custom-name {
    color: #7e9bb5;
    font-size: 12px;
    font-style: italic;
  }
</style>
