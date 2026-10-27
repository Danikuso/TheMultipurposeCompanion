<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, formatShipName, integer, ly } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import ManufacturerIcon from '../../lib/icons/ManufacturerIcon.svelte';
  import StationIcon from '../../lib/icons/StationIcon.svelte';
  import type { AccountsStatusTopic, RemoteShip } from './types';

  const status = $derived(topic<AccountsStatusTopic>('accounts.status'));

  let edsmKey = $state('');
  let inaraKey = $state('');
  let showEdsmKey = $state(false);
  let showInaraKey = $state(false);
  let eddnCopied = $state(false);
  let syncingEdsm = $state(false);
  let syncingInara = $state(false);
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
      type: 'Nave desconocida',
      customName: null,
    };
  }

  $effect(() => {
    if (status?.capi?.connected && !fleetFetched && !loadingFleet) {
      fleetFetched = true;
      fetchFleet();
    }
  });

  async function copyUploaderId() {
    const id = status?.eddn?.uploader_id;
    if (!id) return;
    try {
      await navigator.clipboard.writeText(id);
      eddnCopied = true;
      setTimeout(() => {
        eddnCopied = false;
      }, 2000);
    } catch {
      // Ignorar si clipboard no está disponible
    }
  }

  async function toggleEddn() {
    await action('accounts.set_settings', { eddn_enabled: !status?.eddn?.enabled });
  }

  async function toggleEdsm() {
    await action('accounts.set_settings', { edsm_enabled: !status?.edsm?.enabled });
  }

  async function saveEdsmKey() {
    if (!edsmKey.trim()) return;
    await action('edsm.save_key', {
      api_key: edsmKey.trim(),
    });
    edsmKey = '';
  }

  async function clearEdsmKey() {
    await action('edsm.clear_key', {});
  }

  async function toggleInara() {
    await action('accounts.set_settings', { inara_enabled: !status?.inara?.enabled });
  }

  let testingInara = $state(false);
  let inaraTestFeedback = $state<string | null>(null);

  async function testInaraConnection() {
    testingInara = true;
    inaraTestFeedback = null;
    try {
      const payload: Record<string, string> = {};
      if (inaraKey.trim()) {
        payload.api_key = inaraKey.trim();
      }
      const res = (await action('inara.test_connection', payload)) as { ok?: boolean } | undefined;
      if (res && res.ok) {
        inaraTestFeedback = t('accounts-inara-test-success');
        if (inaraKey.trim()) {
          inaraKey = '';
        }
      } else {
        inaraTestFeedback = t('accounts-inara-test-failed');
      }
    } catch {
      inaraTestFeedback = t('accounts-inara-test-failed');
    } finally {
      testingInara = false;
      setTimeout(() => {
        inaraTestFeedback = null;
      }, 5000);
    }
  }

  async function saveInaraKey() {
    if (!inaraKey.trim()) return;
    await action('inara.save_key', {
      api_key: inaraKey.trim(),
    });
    inaraKey = '';
  }

  async function syncNow() {
    syncingEdsm = true;
    syncingInara = true;
    try {
      await action('accounts.sync_now');
    } finally {
      setTimeout(() => {
        syncingEdsm = false;
        syncingInara = false;
      }, 600);
    }
  }

  async function startFrontierLogin() {
    loginOpening = true;
    try {
      const res = await fetch('/api/auth/frontier/login?open=true', {
        headers: { Accept: 'application/json' },
      });
      if (res.ok) {
        const data = await res.json();
        if (!data.opened && data.url && typeof window !== 'undefined') {
          window.open(data.url, '_blank');
        }
      } else if (typeof window !== 'undefined') {
        window.open('/api/auth/frontier/login', '_blank');
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
</script>

<Panel titleKey="panel-accounts" span={2}>
  <div class="accounts-container">
    <!-- Banner de Política de Privacidad Innegociable -->
    <div class="privacy-banner glass-card" data-testid="accounts-privacy-banner">
      <div class="shield-icon">🛡️</div>
      <div class="banner-content">
        <span class="badge badge-green mono">{t('accounts-title')}</span>
        <p class="privacy-text">{t('accounts-companion-privacy-notice')}</p>
      </div>
    </div>

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
            <span class="badge" class:badge-green={status?.eddn?.connected} class:badge-amber={!status?.eddn?.connected}>
              {status?.eddn?.connected ? t('accounts-eddn-connected') : t('accounts-eddn-disconnected')}
            </span>
            <span class="sub">{t('accounts-eddn-last-upload')} <b class="mono">{status?.eddn?.last_upload_time ?? '—'}</b></span>
          </div>

          <div class="metric-block">
            <span class="metric-label">{t('accounts-eddn-messages-sent')}</span>
            <span class="metric-value mono cyan" data-testid="eddn-messages-count">
              {integer(status?.eddn?.messages_sent_session ?? 0)}
            </span>
          </div>

          <div class="uploader-pill-row">
            <span class="sub mono">{t('accounts-eddn-uploader-id')}:</span>
            <div class="uploader-chip">
              <span class="mono uploader-text" data-testid="eddn-uploader-id">
                {status?.eddn?.uploader_id || 'TMC-Anon'}
              </span>
              <button class="chip-copy-btn" onclick={copyUploaderId} data-testid="eddn-copy-btn" title="Copiar Uploader ID">
                {eddnCopied ? '✓' : '📋'}
              </button>
            </div>
          </div>
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
          <button
            class="switch-btn"
            class:active={status?.edsm?.enabled}
            onclick={toggleEdsm}
            data-testid="edsm-switch"
            aria-label={t('accounts-edsm-switch')}
          >
            <span class="switch-knob"></span>
          </button>
        </div>

        <div class="card-body">
          <div class="status-row">
            <span class="badge" class:badge-green={status?.edsm?.has_key} class:badge-amber={!status?.edsm?.has_key}>
              {status?.edsm?.has_key ? t('accounts-edsm-status-configured') : t('accounts-edsm-status-pending')}
            </span>
            {#if status?.edsm?.commander_name}
              <span class="cmdr-pill mono">CMDR {status.edsm.commander_name}</span>
            {/if}
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
              <button class="action-btn flex-1" onclick={saveEdsmKey} data-testid="edsm-save-btn">
                {t('accounts-edsm-btn-save')}
              </button>
              {#if status?.edsm?.has_key}
                <button class="action-btn btn-secondary flex-1" onclick={clearEdsmKey} data-testid="edsm-clear-btn">
                  {t('accounts-capi-btn-unlink')}
                </button>
              {/if}
            </div>
          </div>

          <div class="card-foot">
            <span class="sub">{t('accounts-edsm-last-sync')} <b class="mono">{status?.edsm?.last_sync_time ?? '—'}</b></span>
            <button class="btn-sync" onclick={syncNow} disabled={syncingEdsm} data-testid="edsm-sync-btn">
              {syncingEdsm ? '...' : t('accounts-edsm-btn-sync')}
            </button>
          </div>
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
          <button
            class="switch-btn"
            class:active={status?.inara?.enabled}
            onclick={toggleInara}
            data-testid="inara-switch"
            aria-label={t('accounts-inara-switch')}
          >
            <span class="switch-knob"></span>
          </button>
        </div>

        <div class="card-body">
          <div class="status-row">
            <div class="badges-wrap">
              <span class="badge" class:badge-green={status?.inara?.has_key} class:badge-amber={!status?.inara?.has_key}>
                {status?.inara?.has_key ? t('accounts-edsm-status-configured') : t('accounts-edsm-status-pending')}
              </span>
              <span class="badge badge-green" data-testid="inara-whitelist-badge" title={t('accounts-inara-whitelist-tooltip')}>
                🛡️ {t('accounts-inara-whitelist-badge')}
              </span>
            </div>
            <div class="badges-wrap">
              {#if status?.inara?.commander_name}
                <span class="cmdr-pill mono">CMDR {status.inara.commander_name}</span>
              {/if}
              {#if status?.inara?.inara_url}
                <a
                  href={status.inara.inara_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="inara-link mono"
                  data-testid="inara-profile-link"
                >
                  🌐 {t('accounts-inara-profile-link')}
                </a>
              {/if}
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
              <button class="action-btn flex-1" onclick={saveInaraKey} data-testid="inara-save-btn">
                {t('accounts-inara-btn-save')}
              </button>
              <button
                class="action-btn btn-secondary flex-1"
                onclick={testInaraConnection}
                disabled={testingInara}
                data-testid="inara-test-btn"
              >
                {testingInara ? t('accounts-inara-test-testing') : t('accounts-inara-btn-test')}
              </button>
            </div>
            {#if inaraTestFeedback}
              <p class="test-feedback mono" data-testid="inara-test-feedback">{inaraTestFeedback}</p>
            {/if}
          </div>

          <!-- Medidor de 2 peticiones/minuto -->
          <div class="rate-limit-card" data-testid="inara-rate-meter">
            <div class="rate-head">
              <span class="sub bold">{t('accounts-inara-rate-limit-title')}</span>
              <span class="sub mono gold">máx 2 req / 60s</span>
            </div>
            <div class="hud-meter-track" role="progressbar" aria-valuenow={1} aria-valuemin={0} aria-valuemax={2}>
              <div class="hud-meter-fill fill-cyan" style:width="50%"></div>
            </div>
            <div class="rate-foot">
              <span class="sub">{t('accounts-inara-pending-batch')} <b class="mono cyan">{status?.inara?.pending_events ?? 0}</b></span>
              <button class="btn-sync" onclick={syncNow} disabled={syncingInara} data-testid="inara-send-btn">
                {syncingInara ? '...' : t('accounts-inara-btn-send')}
              </button>
            </div>
          </div>
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
                        placeholder="https://localhost:47320/api/auth/frontier/callback?code=..."
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
                      <span class="label-text">Nave ({remoteShips.length}):</span>
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
                      <span class="sub mono cyan">Haz clic para seleccionar</span>
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
</Panel>

<style>
  .accounts-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 4px 8px 0;
    max-height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(0, 212, 178, 0.45) rgba(0, 0, 0, 0.2);
  }

  .accounts-container::-webkit-scrollbar {
    width: 6px;
  }

  .accounts-container::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
  }

  .accounts-container::-webkit-scrollbar-thumb {
    background: rgba(0, 212, 178, 0.45);
    border-radius: 3px;
  }

  .accounts-container::-webkit-scrollbar-thumb:hover {
    background: #00d4b2;
  }

  .glass-card {
    background: var(--glass-surface, rgba(14, 22, 33, 0.85));
    border: 1px solid var(--glass-border, rgba(0, 212, 178, 0.2));
    border-radius: 8px;
    backdrop-filter: blur(12px);
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
  }

  .privacy-text {
    font-size: 13px;
    color: var(--text-dim, #8fa3b8);
    margin-top: 4px;
    line-height: 1.4;
  }

  .accounts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 16px;
  }

  .hud-card {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
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
    display: inline-block;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
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
    margin-bottom: 12px;
  }

  .badges-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .metric-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .metric-label {
    font-size: 12px;
    color: var(--text-dim, #7e9bb5);
  }

  .metric-value {
    font-size: 24px;
    font-weight: 700;
    line-height: 1;
  }

  .uploader-pill-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .uploader-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.3);
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .uploader-text {
    font-size: 11px;
    color: #00d4b2;
  }

  .chip-copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 12px;
    padding: 0;
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
    margin-bottom: 12px;
  }

  .input-row {
    display: flex;
    gap: 6px;
  }

  .btn-group-row {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    margin-top: 2px;
  }

  .flex-1 {
    flex: 1;
    text-align: center;
  }

  .glass-input {
    flex: 1;
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

  .inara-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    background: rgba(0, 229, 255, 0.12);
    border: 1px solid rgba(0, 229, 255, 0.35);
    color: #00e5ff;
    text-decoration: none;
    transition: all 0.2s;
  }

  .inara-link:hover {
    background: rgba(0, 229, 255, 0.25);
    border-color: #00e5ff;
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
    justify-content: space-between;
    gap: 8px;
  }

  /* Rate meter */
  .rate-limit-card {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rate-head,
  .rate-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .hud-meter-track {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }

  .hud-meter-fill {
    height: 100%;
    background: #00d4b2;
    transition: width 0.3s;
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
    grid-template-columns: minmax(320px, 360px) 1fr;
    gap: 18px;
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
  }

  .fleet-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
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
  }

  .compact-selector-label .label-text {
    white-space: nowrap;
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.05em;
  }

  .glass-select {
    flex: 1;
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
