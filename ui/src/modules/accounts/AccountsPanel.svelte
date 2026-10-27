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
  let loginOpening = $state(false);
  let remoteShips = $state<RemoteShip[]>([]);
  let selectedShipId = $state<number | null>(null);
  let showFleetGrid = $state(false);
  const activeShip = $derived(remoteShips.find((s) => s.id === selectedShipId) ?? remoteShips[0]);

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
    await action('accounts.set_settings', {
      edsm_has_key: true,
      edsm_enabled: true,
    });
    edsmKey = '';
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
              <button class="action-btn" onclick={saveEdsmKey} data-testid="edsm-save-btn">
                {t('accounts-edsm-btn-save')}
              </button>
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
            <span class="badge" class:badge-green={status?.inara?.has_key} class:badge-amber={!status?.inara?.has_key}>
              {status?.inara?.has_key ? t('accounts-edsm-status-configured') : t('accounts-edsm-status-pending')}
            </span>
            <span class="badge badge-green" data-testid="inara-whitelist-badge">
              🛡️ {t('accounts-inara-whitelist-badge')}
            </span>
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
              <button class="action-btn" onclick={saveInaraKey} data-testid="inara-save-btn">
                {t('accounts-inara-btn-save')}
              </button>
              <button
                class="action-btn btn-secondary"
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

      <!-- 4. TARJETA FRONTIER CAPI -->
      <div class="hud-card glass-card" data-testid="accounts-capi-card">
        <div class="card-head">
          <div class="service-identity">
            <span class="service-icon">🚀</span>
            <div class="service-titles">
              <h4 class="service-name">{t('accounts-capi-title')}</h4>
              <p class="sub">{t('accounts-capi-desc')}</p>
            </div>
          </div>
          <span class="badge" class:badge-green={status?.capi?.connected} class:badge-amber={!status?.capi?.connected}>
            {status?.capi?.connected ? t('accounts-capi-status-linked') : t('accounts-capi-status-unlinked')}
          </span>
        </div>

        <div class="card-body">
          {#if !status?.capi?.connected}
            <div class="link-prompt">
              <p class="sub">{t('accounts-capi-auth-notice')}</p>
              <button class="btn-primary" onclick={startFrontierLogin} disabled={loginOpening} data-testid="capi-link-btn">
                {loginOpening ? t('accounts-capi-btn-opening') : t('accounts-capi-btn-link')}
              </button>
            </div>
          {:else}
            <div class="connected-row">
              <span class="sub">{t('accounts-capi-remote-fleet-count')} <b class="mono cyan">{status?.capi?.remote_ships_count ?? remoteShips.length}</b></span>
              <div class="btn-group">
                <button class="btn-outline" onclick={fetchFleet} disabled={loadingFleet} data-testid="capi-refresh-fleet-btn">
                  {loadingFleet ? '...' : t('accounts-capi-btn-refresh-fleet')}
                </button>
                <button class="btn-danger" onclick={unlinkFrontier} data-testid="capi-unlink-btn">
                  {t('accounts-capi-btn-unlink')}
                </button>
              </div>
            </div>

            <!-- Flota remota -->
            <div class="fleet-list" data-testid="capi-remote-fleet-list">
              {#if remoteShips.length === 0}
                <p class="muted sub">{t('accounts-capi-remote-fleet-empty')}</p>
              {:else}
                <div class="fleet-controls">
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
                        <option value={ship.id}>
                          {formatShipName(ship.ship_type)} {ship.name ? `«${ship.name}»` : ''} - {ship.system}
                        </option>
                      {/each}
                    </select>
                  </label>

                  <button
                    type="button"
                    class="glass-btn-secondary list-toggle-btn"
                    data-testid="toggle-fleet-list"
                    onclick={() => (showFleetGrid = !showFleetGrid)}
                  >
                    {showFleetGrid ? '▲ Ocultar lista' : `▼ Ver flota (${remoteShips.length})`}
                  </button>
                </div>

                {#if activeShip}
                  <div class="active-ship-card">
                    <div class="ship-card-head">
                      <div class="ship-top">
                        <span class="ship-type mono bold">{formatShipName(activeShip.ship_type)}</span>
                        {#if activeShip.name}
                          <span class="ship-custom-name">«{activeShip.name}»</span>
                        {/if}
                      </div>
                      <div class="ship-mfr" title={formatShipName(activeShip.ship_type)}>
                        <ManufacturerIcon shipType={activeShip.ship_type} size={20} />
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

                <div class="fleet-grid" class:is-collapsed={!showFleetGrid}>
                  {#each remoteShips as ship (ship.id)}
                    <div class="ship-card" class:is-selected={ship.id === activeShip?.id}>
                      <div class="ship-card-head">
                        <div class="ship-top">
                          <span class="ship-type mono bold">{formatShipName(ship.ship_type)}</span>
                          {#if ship.name}
                            <span class="ship-custom-name">«{ship.name}»</span>
                          {/if}
                        </div>
                        <div class="ship-mfr" title={formatShipName(ship.ship_type)}>
                          <ManufacturerIcon shipType={ship.ship_type} size={18} />
                        </div>
                      </div>
                      <div class="ship-loc sub">
                        <StationIcon size={13} />
                        <span>{ship.station} ({ship.system})</span>
                      </div>
                      <div class="ship-meta sub mono">
                        <span class="gold">{credits(ship.value)}</span>
                        {#if ship.distance_ly != null}
                          <span>{ly(ship.distance_ly)}</span>
                        {/if}
                      </div>
                    </div>
                  {/each}
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
    margin-bottom: 12px;
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

  /* Fleet list */
  .connected-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .btn-group {
    display: flex;
    gap: 6px;
  }

  .fleet-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    flex-wrap: wrap;
  }

  .compact-selector-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--hud-muted, #7ea0b8);
    flex: 1;
    min-width: 200px;
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
    padding: 4px 8px;
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
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .list-toggle-btn {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.25));
    color: var(--hud-text, #c9d7e5);
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .list-toggle-btn:hover {
    background: rgba(0, 229, 255, 0.12);
    border-color: var(--hud-primary, #00e5ff);
    color: var(--hud-primary, #00e5ff);
  }

  .fleet-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    max-height: 220px;
    overflow-y: auto;
  }

  .fleet-grid.is-collapsed {
    display: none;
  }

  .ship-card {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
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
  }

  .ship-mfr {
    opacity: 0.8;
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
