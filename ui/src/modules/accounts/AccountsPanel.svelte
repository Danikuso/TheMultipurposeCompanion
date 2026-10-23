<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, integer, ly } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
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
  let remoteShips = $state<RemoteShip[]>([]);

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

  async function saveInaraKey() {
    if (!inaraKey.trim()) return;
    await action('accounts.set_settings', {
      inara_has_key: true,
      inara_enabled: true,
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

  function startFrontierLogin() {
    if (typeof window !== 'undefined') {
      window.open('/api/auth/frontier/login', '_blank');
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
            {#if status?.inara?.commander_name}
              <span class="cmdr-pill mono">CMDR {status.inara.commander_name}</span>
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
            </div>
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
              <button class="btn-primary" onclick={startFrontierLogin} data-testid="capi-link-btn">
                {t('accounts-capi-btn-link')}
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
                <div class="fleet-grid">
                  {#each remoteShips as ship (ship.id)}
                    <div class="ship-card">
                      <div class="ship-top">
                        <span class="ship-type mono bold">{ship.ship_type}</span>
                        {#if ship.name}
                          <span class="ship-custom-name">«{ship.name}»</span>
                        {/if}
                      </div>
                      <div class="ship-loc sub">
                        <span>📍 {ship.station} ({ship.system})</span>
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
    padding: 8px 0;
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
    align-items: flex-start;
    gap: 12px;
  }

  .service-identity {
    display: flex;
    align-items: center;
    gap: 12px;
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
    width: 40px;
    height: 22px;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 11px;
    cursor: pointer;
    position: relative;
    padding: 0;
    transition: background 0.2s;
  }

  .switch-btn.active {
    background: #00d4b2;
    border-color: #00d4b2;
  }

  .switch-knob {
    width: 16px;
    height: 16px;
    background: #fff;
    border-radius: 50%;
    position: absolute;
    top: 2px;
    left: 2px;
    transition: transform 0.2s;
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

  .fleet-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    max-height: 220px;
    overflow-y: auto;
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

  .ship-top,
  .ship-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
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
