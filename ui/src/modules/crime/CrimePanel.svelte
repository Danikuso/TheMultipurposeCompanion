<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, ly } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { CrimeStatusTopic } from './types';

  const status = $derived(topic<CrimeStatusTopic>('crime.status'));
  const loc = $derived(topic<{ mode?: string }>('core.location'));
  const isDocked = $derived(loc?.mode === 'Docked');

  let copiedFactor = $state(false);
  let copiedMarket = $state(false);

  async function copyToClipboard(text: string, type: 'factor' | 'market') {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      if (type === 'factor') {
        copiedFactor = true;
        setTimeout(() => (copiedFactor = false), 2000);
      } else {
        copiedMarket = true;
        setTimeout(() => (copiedMarket = false), 2000);
      }
    }
  }

  const legalStateLabel = $derived.by(() => {
    const ls = status?.legal_state ?? 'Clean';
    switch (ls) {
      case 'Clean':
        return t('crime-state-clean');
      case 'Wanted':
        return t('crime-state-wanted');
      case 'IllegalCargo':
        return t('crime-state-illegal-cargo');
      case 'Hostile':
        return t('crime-state-hostile');
      default:
        return ls;
    }
  });

  const legalStateBadgeClass = $derived.by(() => {
    const ls = status?.legal_state ?? 'Clean';
    switch (ls) {
      case 'Clean':
        return 'badge-green';
      case 'Wanted':
      case 'Hostile':
        return 'badge-danger';
      case 'IllegalCargo':
        return 'badge-orange';
      default:
        return 'badge-cyan';
    }
  });

  const notoriety = $derived(status?.notoriety ?? 0);
  const decaySeconds = $derived(status?.notoriety_decay_seconds ?? 0);
  const decayHours = $derived(Math.floor(decaySeconds / 3600));
  const decayMinutes = $derived(Math.floor((decaySeconds % 3600) / 60));

  const totalFines = $derived(status?.total_fines ?? 0);
  const totalBounties = $derived(status?.total_bounties ?? 0);
  const hasFinesOrBounties = $derived(totalFines > 0 || totalBounties > 0);

  const securityLevel = $derived(status?.system_security ?? 'unknown');
  const securityBadgeClass = $derived.by(() => {
    switch (securityLevel) {
      case 'high':
        return 'badge-green';
      case 'medium':
        return 'badge-cyan';
      case 'low':
        return 'badge-orange';
      case 'anarchy':
        return 'badge-danger';
      default:
        return '';
    }
  });

  const securityLabel = $derived.by(() => {
    switch (securityLevel) {
      case 'high':
        return t('crime-security-high');
      case 'medium':
        return t('crime-security-medium');
      case 'low':
        return t('crime-security-low');
      case 'anarchy':
        return t('crime-security-anarchy');
      default:
        return securityLevel;
    }
  });

  const policeResponseLabel = $derived.by(() => {
    switch (securityLevel) {
      case 'high':
        return t('crime-police-response-fast');
      case 'medium':
        return t('crime-police-response-moderate');
      case 'low':
        return t('crime-police-response-slow');
      case 'anarchy':
        return t('crime-police-response-none');
      default:
        return '—';
    }
  });
</script>

<Panel titleKey="crime-title" span={2}>
  <div class="crime-panel" data-testid="crime-panel">
    <!-- Fila Superior: Estado Legal y Medidor de Notoriedad -->
    <div class="hud-grid-2">
      <!-- Tarjeta 1: Estado Legal y Notoriedad -->
      <div class="hud-card" data-testid="crime-status-card">
        <div class="card-head">
          <span class="card-title">{t('crime-card-status')}</span>
          <span class="badge {legalStateBadgeClass}" data-testid="legal-state-badge">
            {legalStateLabel}
          </span>
        </div>

        <div class="notoriety-section">
          <div class="notoriety-head">
            <span class="metric-label">{t('crime-notoriety')}</span>
            <span
              class="notoriety-val mono bold {notoriety > 0 ? 'danger' : 'green'}"
              data-testid="notoriety-level"
            >
              {#if notoriety > 0}
                {t('crime-notoriety-level', { level: notoriety })}
              {:else}
                {t('crime-notoriety-clean')}
              {/if}
            </span>
          </div>

          <!-- Medidor visual de 10 segmentos de notoriedad -->
          <div
            class="notoriety-bar-track"
            data-testid="notoriety-meter"
            role="progressbar"
            aria-valuenow={notoriety}
            aria-valuemin={0}
            aria-valuemax={10}
          >
            {#each Array(10) as _, i}
              <div
                class="meter-segment"
                class:active={i < notoriety}
                class:high={i >= 6}
                class:mid={i >= 3 && i < 6}
              ></div>
            {/each}
          </div>

          {#if notoriety > 0}
            <div class="decay-info" data-testid="decay-timer">
              {#if isDocked}
                <div class="decay-paused orange">
                  <span class="decay-icon">⏸️</span>
                  <span>{t('crime-notoriety-decay-paused')}</span>
                </div>
              {:else}
                <div class="decay-active cyan">
                  <span class="decay-icon">⏳</span>
                  <span>{t('crime-notoriety-decay-active', { hours: decayHours, minutes: decayMinutes })}</span>
                </div>
              {/if}
              <p class="sub muted decay-note">{t('crime-notoriety-decay-note')}</p>
            </div>
          {/if}
        </div>
      </div>

      <!-- Tarjeta 2: Guía de Limpieza de Nombre del Ayudante -->
      <div class="hud-card guide-card" data-testid="crime-guide-card">
        <div class="card-head">
          <span class="card-title">{t('crime-card-guide')}</span>
        </div>

        {#if notoriety > 0}
          <!-- Caso Bloqueado: Notoriedad > 0 -->
          <div class="guide-blocked" data-testid="guide-blocked">
            <div class="guide-banner banner-danger">
              <span class="banner-icon">⛔</span>
              <div class="banner-text">
                <strong class="banner-title">{t('crime-guide-blocked-title')}</strong>
                <p class="banner-desc">{t('crime-guide-blocked-desc')}</p>
              </div>
            </div>
          </div>
        {:else if hasFinesOrBounties}
          <!-- Caso Disponible: Notoriedad 0 y multas/recompensas pendientes -->
          <div class="guide-ready" data-testid="guide-ready">
            <div class="guide-banner banner-success">
              <span class="banner-icon">✅</span>
              <div class="banner-text">
                <strong class="banner-title">{t('crime-guide-ready-title')}</strong>
                <p class="banner-desc">{t('crime-guide-ready-desc')}</p>
              </div>
            </div>

            {#if status?.nearest_interstellar_factor}
              <div class="facility-card" data-testid="nearest-factor-card">
                <div class="facility-head">
                  <span class="facility-type">{t('crime-nearest-factor')}</span>
                  <span class="badge badge-cyan">{status.nearest_interstellar_factor.pad_size} PAD</span>
                </div>
                <div class="facility-body">
                  <div class="facility-info">
                    <span class="facility-station bold">{status.nearest_interstellar_factor.station_name}</span>
                    <span class="facility-system mono sub">
                      {status.nearest_interstellar_factor.system_name} ({ly(status.nearest_interstellar_factor.distance_ly)})
                    </span>
                  </div>
                  <button
                    type="button"
                    class="btn-copy"
                    data-testid="copy-interstellar-factor-btn"
                    onclick={() => copyToClipboard(status.nearest_interstellar_factor!.system_name, 'factor')}
                  >
                    {copiedFactor ? t('crime-system-copied') : t('crime-copy-system')}
                  </button>
                </div>
              </div>
            {:else}
              <p class="muted sub">{t('crime-no-facility')}</p>
            {/if}
          </div>
        {:else}
          <!-- Caso Limpio: Todo en regla -->
          <div class="guide-clean" data-testid="guide-clean">
            <div class="guide-banner banner-info">
              <span class="banner-icon">🛡️</span>
              <div class="banner-text">
                <strong class="banner-title">{t('crime-guide-all-clean')}</strong>
                <p class="banner-desc">{t('crime-guide-all-clean-desc')}</p>
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Fila Central: Contrabando, Mercados Negros y Consejos de Sigilo -->
    <div class="hud-grid-2">
      <!-- Tarjeta 3: Contrabando y Mercados Negros -->
      <div class="hud-card" data-testid="crime-contraband-card">
        <div class="card-head">
          <span class="card-title">{t('crime-card-contraband')}</span>
        </div>

        <div class="stolen-cargo-section">
          <span class="metric-label">{t('crime-stolen-cargo')}</span>
          {#if (status?.stolen_items?.length ?? 0) > 0}
            <ul class="stolen-list" data-testid="stolen-items-list">
              {#each status!.stolen_items as item}
                <li class="stolen-item">
                  <span class="item-name bold">{item.name_localised ?? item.name}</span>
                  <span class="badge badge-orange">{t('crime-stolen-units', { count: item.stolen })}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="muted sub">{t('crime-stolen-none')}</p>
          {/if}
        </div>

        <!-- Mercado Negro cercano -->
        <div class="black-market-section">
          {#if status?.nearest_black_market}
            <div class="facility-card" data-testid="nearest-market-card">
              <div class="facility-head">
                <span class="facility-type">{t('crime-nearest-black-market')}</span>
                <span class="badge badge-orange">{status.nearest_black_market.pad_size} PAD</span>
              </div>
              <div class="facility-body">
                <div class="facility-info">
                  <span class="facility-station bold">{status.nearest_black_market.station_name}</span>
                  <span class="facility-system mono sub">
                    {status.nearest_black_market.system_name} ({ly(status.nearest_black_market.distance_ly)})
                  </span>
                </div>
                <button
                  type="button"
                  class="btn-copy"
                  data-testid="copy-black-market-btn"
                  onclick={() => copyToClipboard(status.nearest_black_market!.system_name, 'market')}
                >
                  {copiedMarket ? t('crime-system-copied') : t('crime-copy-system')}
                </button>
              </div>
            </div>
          {:else}
            <p class="muted sub">{t('crime-no-facility')}</p>
          {/if}
        </div>

        <!-- Consejos de Aproximación Sigilosa -->
        <div class="stealth-advice">
          <span class="metric-label">{t('crime-stealth-advice-title')}</span>
          <div class="stepper">
            <div class="step-item">
              <span class="step-num mono">1</span>
              <span class="step-text sub">{t('crime-stealth-advice-step1')}</span>
            </div>
            <div class="step-item">
              <span class="step-num mono">2</span>
              <span class="step-text sub">{t('crime-stealth-advice-step2')}</span>
            </div>
            <div class="step-item">
              <span class="step-num mono">3</span>
              <span class="step-text sub">{t('crime-stealth-advice-step3')}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Tarjeta 4 & 5: Facciones, Penas y Seguridad del Sistema -->
      <div class="factions-security-column">
        <!-- Tarjeta 4: Facciones y Multas -->
        <div class="hud-card" data-testid="crime-factions-card">
          <div class="card-head">
            <span class="card-title">{t('crime-card-factions')}</span>
          </div>

          <div class="totals-grid">
            <div class="metric-box">
              <span class="metric-label">{t('crime-total-fines')}</span>
              <span class="metric-val mono bold {totalFines > 0 ? 'orange' : 'muted'}" data-testid="total-fines">
                {credits(totalFines)}
              </span>
            </div>
            <div class="metric-box">
              <span class="metric-label">{t('crime-total-bounties')}</span>
              <span class="metric-val mono bold {totalBounties > 0 ? 'danger' : 'muted'}" data-testid="total-bounties">
                {credits(totalBounties)}
              </span>
            </div>
          </div>

          <div class="factions-list-container" data-testid="factions-list">
            {#if (status?.factions?.length ?? 0) > 0}
              <ul class="factions-list">
                {#each status!.factions as fac}
                  <li class="faction-row">
                    <div class="faction-meta">
                      <span class="faction-name bold">{fac.faction}</span>
                      {#if fac.system}
                        <span class="sub mono">{fac.system}</span>
                      {/if}
                    </div>
                    <div class="faction-amounts">
                      {#if fac.fines > 0}
                        <span class="badge badge-orange">{t('crime-faction-fines', { amount: fac.fines })}</span>
                      {/if}
                      {#if fac.bounties > 0}
                        <span class="badge badge-danger">{t('crime-faction-bounties', { amount: fac.bounties })}</span>
                      {/if}
                    </div>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="muted sub">{t('crime-no-fines-bounties')}</p>
            {/if}
          </div>
        </div>

        <!-- Tarjeta 5: Termómetro de Seguridad del Sistema -->
        <div class="hud-card" data-testid="crime-security-card">
          <div class="card-head">
            <span class="card-title">{t('crime-card-security')}</span>
            <span class="badge {securityBadgeClass}" data-testid="security-badge">
              {securityLabel}
            </span>
          </div>

          <div class="security-info">
            <span class="metric-label">{t('crime-card-security')}</span>
            <p class="police-desc sub" data-testid="police-response">
              {policeResponseLabel}
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</Panel>

<style>
  .crime-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 0.25rem 0;
  }

  .hud-grid-2 {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 1rem;
  }

  .factions-security-column {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .hud-card {
    background: var(--hud-surface-card, rgba(16, 24, 39, 0.7));
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: var(--hud-radius-md, 8px);
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    padding-bottom: 0.5rem;
  }

  .card-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--hud-text-bright, #f3f4f6);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Notoriedad */
  .notoriety-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .notoriety-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .notoriety-val {
    font-size: 1.1rem;
  }

  .notoriety-bar-track {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    gap: 4px;
    height: 10px;
    background: rgba(0, 0, 0, 0.3);
    padding: 2px;
    border-radius: 4px;
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.1));
  }

  .meter-segment {
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
    transition: background 0.2s ease;
  }

  .meter-segment.active {
    background: #10b981;
    box-shadow: 0 0 4px rgba(16, 185, 129, 0.4);
  }

  .meter-segment.active.mid {
    background: #f59e0b;
    box-shadow: 0 0 4px rgba(245, 158, 11, 0.4);
  }

  .meter-segment.active.high {
    background: #ef4444;
    box-shadow: 0 0 6px rgba(239, 68, 68, 0.6);
  }

  .decay-info {
    margin-top: 0.25rem;
    padding: 0.5rem 0.75rem;
    background: rgba(0, 0, 0, 0.25);
    border-radius: 6px;
  }

  .decay-paused, .decay-active {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .decay-note {
    margin-top: 0.25rem;
    font-size: 0.75rem;
  }

  /* Guía de limpieza */
  .guide-banner {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 6px;
  }

  .banner-icon {
    font-size: 1.3rem;
  }

  .banner-danger {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .banner-success {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .banner-info {
    background: rgba(6, 182, 212, 0.12);
    border: 1px solid rgba(6, 182, 212, 0.3);
  }

  .banner-title {
    display: block;
    font-size: 0.85rem;
    margin-bottom: 0.25rem;
  }

  .banner-desc {
    font-size: 0.8rem;
    color: var(--hud-text-muted, #9ca3af);
    line-height: 1.35;
    margin: 0;
  }

  /* Instalación cercana (Interstellar Factor / Black Market) */
  .facility-card {
    margin-top: 0.75rem;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .facility-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .facility-type {
    font-size: 0.75rem;
    text-transform: uppercase;
    color: var(--hud-text-muted, #9ca3af);
  }

  .facility-body {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .facility-info {
    display: flex;
    flex-direction: column;
  }

  .facility-station {
    color: var(--hud-text-bright, #fff);
    font-size: 0.95rem;
  }

  .btn-copy {
    background: var(--hud-surface-button, rgba(255, 255, 255, 0.08));
    color: var(--hud-text-bright, #f3f4f6);
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.15));
    border-radius: 4px;
    padding: 0.35rem 0.75rem;
    font-size: 0.75rem;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .btn-copy:hover {
    background: var(--hud-surface-button-hover, rgba(255, 255, 255, 0.15));
    border-color: var(--hud-accent-cyan, #06b6d4);
  }

  /* Contrabando */
  .stolen-list {
    list-style: none;
    padding: 0;
    margin: 0.25rem 0 0.75rem 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .stolen-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.5rem;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
  }

  /* Stepper de sigilo */
  .stealth-advice {
    margin-top: 0.5rem;
    border-top: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    padding-top: 0.5rem;
  }

  .stepper {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.35rem;
  }

  .step-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .step-num {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: rgba(6, 182, 212, 0.15);
    color: var(--hud-accent-cyan, #06b6d4);
    border: 1px solid rgba(6, 182, 212, 0.3);
    font-size: 0.75rem;
    font-weight: bold;
    flex-shrink: 0;
  }

  /* Facciones y Totales */
  .totals-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .metric-box {
    background: rgba(0, 0, 0, 0.2);
    padding: 0.5rem;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .factions-list {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .faction-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.5rem;
    background: rgba(0, 0, 0, 0.15);
    border-radius: 4px;
  }

  .faction-meta {
    display: flex;
    flex-direction: column;
  }

  .faction-amounts {
    display: flex;
    gap: 0.35rem;
  }

  /* Badges y utilidades */
  .badge {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge-green {
    background: rgba(16, 185, 129, 0.15);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .badge-danger {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .badge-orange {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .badge-cyan {
    background: rgba(6, 182, 212, 0.15);
    color: #06b6d4;
    border: 1px solid rgba(6, 182, 212, 0.3);
  }

  .metric-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--hud-text-muted, #9ca3af);
  }

  .bold { font-weight: 600; }
  .mono { font-family: var(--hud-font-mono, monospace); }
  .sub { font-size: 0.8rem; }
  .muted { color: var(--hud-text-muted, #9ca3af); }
  .danger { color: #ef4444; }
  .orange { color: #f59e0b; }
  .green { color: #10b981; }
  .cyan { color: #06b6d4; }
</style>
