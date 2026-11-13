<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, formatShipName, integer, ly } from '../../lib/format';
  import { action } from '../../lib/ws.svelte';
  import ManufacturerIcon from '../../lib/icons/ManufacturerIcon.svelte';
  import StationIcon from '../../lib/icons/StationIcon.svelte';
  import type { CapiStatus, RemoteShip } from './types';

  let { capi }: { capi: CapiStatus | null } = $props();

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
    if (capi?.connected && !fleetFetched && !loadingFleet) {
      fleetFetched = true;
      fetchFleet();
    }
  });

  const officialRedirectUri = 'https://danikuso.github.io/TheMultipurposeCompanion/oauth/callback.html';

  async function startFrontierLogin() {
    loginOpening = true;
    try {
      const res = await fetch(
        `/api/auth/frontier/login?open=true&redirect_uri=${encodeURIComponent(officialRedirectUri)}`,
        { headers: { Accept: 'application/json' } }
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
      // Ignorar si no está vinculada
    } finally {
      loadingFleet = false;
    }
  }
</script>

<div class="hud-card glass-card capi-card" data-testid="accounts-capi-card">
  <div class="card-head capi-card-head">
    <div class="service-identity">
      <span class="service-icon" aria-hidden="true">🚀</span>
      <div class="service-titles">
        <div class="capi-title-row">
          <h4 class="service-name">{t('accounts-capi-title')}</h4>
          <span class="badge" class:badge-green={capi?.connected} class:badge-amber={!capi?.connected}>
            {capi?.connected ? t('accounts-capi-status-linked') : t('accounts-capi-status-unlinked')}
          </span>
          {#if capi?.connected && capi?.commander_name}
            <span class="cmdr-pill mono">CMDR {capi.commander_name}</span>
          {/if}
        </div>
        <p class="sub">{t('accounts-capi-desc')}</p>
      </div>
    </div>
    {#if capi?.connected}
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
    {#if !capi?.connected}
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
      <div class="fleet-container" data-testid="capi-remote-fleet-list">
        {#if remoteShips.length === 0}
          <div class="capi-empty-state">
            <div class="capi-stats-bar">
              <div class="stat-item">
                <span class="sub">{t('accounts-capi-remote-fleet-count')}</span>
                <span class="mono cyan bold stat-val">{integer(capi?.remote_ships_count ?? 0)}</span>
              </div>
              {#if capi?.last_sync_time}
                <div class="stat-item">
                  <span class="sub">{t('accounts-edsm-last-sync')}</span>
                  <span class="mono stat-val">{capi.last_sync_time}</span>
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

<style>
  .glass-card {
    background: var(--hud-surface, rgba(14, 22, 33, 0.95));
    border: 1px solid var(--glass-border, rgba(0, 212, 178, 0.2));
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .capi-card {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 16px;
  }

  .service-identity {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }

  .service-icon {
    font-size: 20px;
    line-height: 1.2;
    flex-shrink: 0;
  }

  .service-titles {
    min-width: 0;
    flex: 1;
  }

  .capi-title-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .service-name {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: var(--hud-text, #f0f4f8);
  }

  .sub {
    margin: 3px 0 0;
    font-size: 12px;
    color: var(--hud-muted, #94a3b8);
    line-height: 1.35;
  }

  .badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 4px;
  }

  .badge-green { background: rgba(16, 185, 129, 0.16); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.35); }
  .badge-amber { background: rgba(245, 158, 11, 0.16); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.35); }

  .cmdr-pill {
    font-size: 11px;
    font-weight: 700;
    color: #00d4b2;
    background: rgba(0, 212, 178, 0.1);
    border: 1px solid rgba(0, 212, 178, 0.3);
    padding: 2px 8px;
    border-radius: 4px;
  }

  .mono { font-family: var(--font-mono, ui-monospace, monospace); }
  .cyan { color: var(--hud-info, #38bdf8); }
  .gold { color: #fbbf24; }
  .bold { font-weight: 700; }

  .btn-group, .btn-group-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .btn-primary, .btn-outline, .btn-danger, .action-btn {
    border-radius: 5px;
    padding: 7px 12px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .btn-primary { background: rgba(0, 212, 178, 0.2); color: #00d4b2; border-color: rgba(0, 212, 178, 0.45); }
  .btn-outline { background: transparent; color: #cbd5e1; border-color: rgba(148, 163, 184, 0.35); }
  .btn-danger { background: rgba(239, 68, 68, 0.15); color: #f87171; border-color: rgba(239, 68, 68, 0.35); }
  .action-btn { background: rgba(56, 189, 248, 0.18); color: #38bdf8; border-color: rgba(56, 189, 248, 0.4); }

  .link-prompt {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .manual-toggle-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: #94a3b8;
    font-size: 11px;
  }

  .manual-input-box {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .glass-input, .glass-select {
    flex: 1 1 200px;
    min-width: 0;
    background: rgba(10, 15, 25, 0.75);
    color: #f0f4f8;
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 5px;
    padding: 7px 10px;
    font-size: 12px;
  }

  .capi-empty-state {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .capi-stats-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .capi-info-box {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 6px;
    background: rgba(56, 189, 248, 0.06);
    border: 1px solid rgba(56, 189, 248, 0.25);
  }

  .info-empty-title, .info-empty-desc {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
  }

  .info-empty-title { color: #e2e8f0; font-weight: 600; }
  .info-empty-desc { color: #94a3b8; margin-top: 3px; }

  .fleet-cockpit-layout {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(260px, 1.4fr);
    gap: 12px;
  }

  .fleet-detail-col, .fleet-grid-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .compact-selector-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    color: #94a3b8;
  }

  .fleet-grid-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
  }

  .fleet-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 8px;
  }

  .active-ship-card, .ship-card {
    text-align: left;
    background: rgba(10, 15, 25, 0.65);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: inherit;
    cursor: pointer;
  }

  .ship-card.is-selected {
    border-color: #00d4b2;
    background: rgba(0, 212, 178, 0.08);
  }

  .ship-card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .ship-top {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
  }

  .ship-type { font-size: 12px; color: #f0f4f8; }
  .ship-custom-name { font-size: 11px; color: #94a3b8; }
  .ship-loc, .ship-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; font-size: 11px; }

  @media (max-width: 820px) {
    .fleet-cockpit-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
