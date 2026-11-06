<script lang="ts">
  import { t } from '../../lib/i18n';
  import { prefs, setHomeBase } from '../../lib/prefs.svelte';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { pushToast } from '../../shell/Toasts.svelte';

  interface LocationTopic {
    system?: string;
    system_address?: number;
    star_pos?: [number, number, number];
    body?: string | null;
    body_type?: string | null;
    body_id?: number | null;
    station?: string | null;
    station_type?: string | null;
    market_id?: number | null;
    mode?: string;
    population?: number | null;
    allegiance?: string | null;
    economy?: string | null;
    second_economy?: string | null;
    government?: string | null;
    security?: string | null;
    controlling_faction?: string | null;
  }

  interface StatusTopic {
    LegalState?: string;
    legal_state?: string;
    Latitude?: number;
    latitude?: number;
    Longitude?: number;
    longitude?: number;
    Altitude?: number;
    altitude?: number;
    Heading?: number;
    heading?: number;
  }

  const MODE_KEY_MAP: Record<string, string> = {
    Docked: 'mode-docked',
    Landed: 'mode-landed',
    NormalSpace: 'mode-normal-space',
    Supercruise: 'mode-supercruise',
    Hyperspace: 'mode-hyperspace',
    OnFoot: 'mode-on-foot',
    InSrv: 'mode-in-srv',
    InFighter: 'mode-in-fighter',
    InTaxi: 'mode-in-taxi',
    Multicrew: 'mode-multicrew',
    Unknown: 'mode-unknown',
  };

  const loc = $derived(topic<LocationTopic>('core.location'));
  const status = $derived(topic<StatusTopic>('core.status'));

  const modeKey = $derived(MODE_KEY_MAP[loc?.mode ?? 'Unknown'] ?? 'mode-unknown');

  const coords = $derived(loc?.star_pos ?? null);
  const distSol = $derived(
    coords ? Math.hypot(coords[0], coords[1], coords[2]).toFixed(1) : null
  );

  const isCurrentSystemHome = $derived(
    Boolean(
      loc?.system &&
      prefs.homeSystem &&
      loc.system.trim().toLowerCase() === prefs.homeSystem.trim().toLowerCase()
    )
  );

  const distBase = $derived.by(() => {
    if (!coords || !prefs.homeSystemCoords || prefs.homeSystemCoords.length < 3) return null;
    const [x1, y1, z1] = coords;
    const [x2, y2, z2] = prefs.homeSystemCoords;
    return Math.hypot(x1 - x2, y1 - y2, z1 - z2).toFixed(1);
  });

  function toggleHomeBase() {
    if (!loc?.system) return;
    if (isCurrentSystemHome) {
      void setHomeBase(null, null);
      pushToast(t('location-home-base-cleared'), 'Info');
    } else {
      const starPos = loc.star_pos ?? (loc.system.toLowerCase() === 'sol' ? [0, 0, 0] : null);
      void setHomeBase(loc.system, starPos);
      pushToast(t('location-home-base-set', { system: loc.system }), 'Info');
    }
  }

  const legalState = $derived(status?.legal_state ?? status?.LegalState ?? null);
  const lat = $derived(status?.latitude ?? status?.Latitude ?? null);
  const lon = $derived(status?.longitude ?? status?.Longitude ?? null);
  const alt = $derived(status?.altitude ?? status?.Altitude ?? null);
  const heading = $derived(status?.heading ?? status?.Heading ?? null);

  let copied = $state(false);
  function copySystemName() {
    if (!loc?.system) return;
    navigator.clipboard?.writeText(loc.system);
    copied = true;
    pushToast(t('action-copied'), 'Info');
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  function formatPopulation(pop?: number | null): string {
    if (pop == null) return '—';
    if (pop === 0) return t('population-unpopulated');
    if (pop >= 1_000_000_000) return `${(pop / 1_000_000_000).toFixed(2)} B hab.`;
    if (pop >= 1_000_000) return `${(pop / 1_000_000).toFixed(1)} M hab.`;
    if (pop >= 1_000) return `${(pop / 1_000).toFixed(1)} k hab.`;
    return `${pop} hab.`;
  }

  function cleanVal(v?: string | null): string | null {
    if (!v) return null;
    let s = v.trim();
    if (s.startsWith('$') && s.endsWith(';')) {
      s = s.slice(1, -1);
      const idx = s.indexOf('_');
      if (idx !== -1) s = s.slice(idx + 1);
    }
    return s || null;
  }

  function formatSecurity(sec?: string | null): string | null {
    if (!sec) return null;
    let raw = sec.trim();
    if (raw.startsWith('$') && raw.endsWith(';')) {
      raw = raw.slice(1, -1);
    }
    raw = raw.toLowerCase().replace('system_security_', '').replace('security_', '');
    const key = `security-${raw}`;
    const translated = t(key);
    return translated !== key ? translated : cleanVal(sec);
  }
</script>

<Panel titleKey="panel-location">
  {#if !loc || !loc.system}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="loc-container">
      <div class="system-header-card">
        <div class="system-main">
          <div class="system-title-row">
            <span class="label">{t('field-system')}</span>
            <strong class="system-name mono font-mono" data-testid="system-name">{loc.system}</strong>
            <button
              type="button"
              class="copy-btn"
              title={t('action-copy-system')}
              onclick={copySystemName}
            >
              {copied ? '✓' : '📋'}
            </button>
            <button
              type="button"
              class="home-btn"
              class:is-home={isCurrentSystemHome}
              title={isCurrentSystemHome ? t('location-home-base-active') : t('location-set-home-base')}
              onclick={toggleHomeBase}
              data-testid="toggle-home-base-btn"
            >
              {isCurrentSystemHome ? '🏠' : '📍'}
            </button>
            {#if isCurrentSystemHome}
              <span class="badge home-badge">{t('field-home-base')}</span>
            {/if}
          </div>
          {#if coords}
            <div class="coords-row font-mono" title="{t('field-coordinates')}: [{coords[0].toFixed(1)}, {coords[1].toFixed(1)}, {coords[2].toFixed(1)}] {t('unit-light-year')}">
              <span class="coords-val">[{coords[0].toFixed(1)}, {coords[1].toFixed(1)}, {coords[2].toFixed(1)}] {t('unit-light-year')}</span>
              {#if isCurrentSystemHome}
                <span class="dist-base base-current">· 🏠 {t('field-home-base')}</span>
              {:else if distBase != null && prefs.homeSystem}
                <span class="dist-base" data-testid="dist-base">· {distBase} {t('unit-light-year')} {t('field-distance-base', { system: prefs.homeSystem })}</span>
              {/if}
              {#if distSol && (!prefs.homeSystem || prefs.homeSystem.toLowerCase() !== 'sol')}
                <span class="dist-sol">· {distSol} {t('unit-light-year')} {t('field-distance-sol')}</span>
              {/if}
            </div>
          {/if}
        </div>

        <div class="badges-group">
          <span class="mode-badge" data-testid="location-mode">{t(modeKey)}</span>
          {#if legalState}
            <span
              class="legal-badge"
              class:legal-clean={legalState === 'Clean'}
              class:legal-wanted={legalState === 'Wanted'}
              class:legal-lawless={legalState === 'Lawless' || legalState === 'Anarchy'}
            >
              {t(`legal-${legalState.toLowerCase()}`) !== `legal-${legalState.toLowerCase()}` ? t(`legal-${legalState.toLowerCase()}`) : legalState}
            </span>
          {/if}
        </div>
      </div>

      <div class="loc-details-grid">
        {#if loc.allegiance || loc.security}
          <div class="detail-row">
            <span class="detail-label">{t('field-security')} / {t('field-allegiance')}</span>
            <div class="detail-val-group">
              {#if loc.security}
                <span class="security-pill">{formatSecurity(loc.security)}</span>
              {/if}
              {#if loc.allegiance}
                <span class="allegiance-pill">{cleanVal(loc.allegiance)}</span>
              {/if}
            </div>
          </div>
        {/if}

        {#if loc.economy || loc.government}
          <div class="detail-row">
            <span class="detail-label">{t('field-government')} / {t('field-economy')}</span>
            <span class="detail-val">
              {cleanVal(loc.government) || '—'} · {cleanVal(loc.economy) || '—'}
              {#if loc.second_economy && cleanVal(loc.second_economy) !== 'None' && cleanVal(loc.second_economy) !== 'Ninguna'}
                / {cleanVal(loc.second_economy)}
              {/if}
            </span>
          </div>
        {/if}

        {#if loc.population != null || loc.controlling_faction}
          <div class="detail-row">
            <span class="detail-label">{t('field-population')}</span>
            <span class="detail-val">
              {formatPopulation(loc.population)}
              {#if loc.controlling_faction}
                <span class="faction-sub">({loc.controlling_faction})</span>
              {/if}
            </span>
          </div>
        {/if}

        <div class="detail-row body-row">
          <span class="detail-label">{t('field-body')}</span>
          <span class="detail-val font-mono">
            {loc.body || '—'}
            {#if loc.body_type}
              <span class="dim">({loc.body_type})</span>
            {/if}
          </span>
        </div>

        {#if loc.station}
          <div class="detail-row station-row">
            <span class="detail-label">{t('field-station')}</span>
            <span class="detail-val font-mono">
              {loc.station}
              {#if loc.station_type}
                <span class="dim">({loc.station_type})</span>
              {/if}
            </span>
          </div>
        {/if}

        {#if lat != null && lon != null}
          <div class="surface-card">
            <span class="surface-title">{t('field-surface-coords')}</span>
            <div class="surface-coords font-mono">
              <span>Lat: {lat.toFixed(4)}°</span>
              <span>Lon: {lon.toFixed(4)}°</span>
              {#if alt != null}
                <span>Alt: {Math.round(alt)} m</span>
              {/if}
              {#if heading != null}
                <span>{t('field-heading')}: {heading}°</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .loc-container {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .system-header-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .system-main {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .system-title-row {
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .label {
    color: var(--hud-muted);
    font-size: 0.75rem;
  }

  .system-name {
    font-size: 1.05rem;
    color: var(--hud-primary);
  }

  .copy-btn {
    background: transparent;
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.75rem;
    cursor: pointer;
    line-height: 1;
    transition: all 120ms ease;
  }

  .copy-btn:hover {
    border-color: var(--hud-primary);
    color: var(--hud-primary);
  }

  .home-btn {
    background: transparent;
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    font-size: 0.75rem;
    cursor: pointer;
    line-height: 1;
    transition: all 120ms ease;
  }

  .home-btn:hover {
    border-color: var(--hud-primary);
    color: var(--hud-primary);
  }

  .home-btn.is-home {
    border-color: var(--hud-cyan, #38bdf8);
    background: rgba(56, 189, 248, 0.12);
  }

  .home-badge {
    font-size: 0.68rem;
    padding: 0.08rem 0.35rem;
    border-radius: 3px;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid var(--hud-cyan, #38bdf8);
    color: var(--hud-cyan, #38bdf8);
    font-weight: 600;
  }

  .dist-base {
    color: var(--hud-cyan, #38bdf8);
    font-weight: 600;
  }

  .base-current {
    color: var(--hud-success, #34d399);
  }

  .coords-row {
    font-size: 0.72rem;
    color: var(--hud-muted);
  }

  .dist-sol {
    color: var(--hud-secondary);
  }

  .badges-group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .mode-badge {
    font-size: 0.78rem;
    padding: 0.15rem 0.45rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border);
    color: var(--hud-secondary);
    background: rgba(255, 255, 255, 0.03);
  }

  .legal-badge {
    font-size: 0.72rem;
    padding: 0.12rem 0.4rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
  }

  .legal-clean {
    color: var(--hud-success, #22c55e);
    border-color: rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.1);
  }

  .legal-wanted {
    color: var(--hud-danger, #ef4444);
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(239, 68, 68, 0.1);
  }

  .legal-lawless {
    color: #fbbf24;
    border-color: rgba(251, 191, 36, 0.4);
    background: rgba(251, 191, 36, 0.1);
  }

  .loc-details-grid {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 0.8rem;
    gap: 0.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    padding-bottom: 0.25rem;
  }

  .detail-label {
    color: var(--hud-muted);
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .detail-val {
    color: var(--hud-text);
    text-align: right;
  }

  .detail-val-group {
    display: flex;
    gap: 0.35rem;
  }

  .security-pill, .allegiance-pill {
    font-size: 0.72rem;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border-subtle);
    background: rgba(255, 255, 255, 0.02);
  }

  .faction-sub {
    color: var(--hud-muted);
    font-size: 0.72rem;
    margin-left: 0.25rem;
  }

  .dim {
    color: var(--hud-muted);
    font-size: 0.75rem;
  }

  .surface-card {
    background: rgba(56, 189, 248, 0.04);
    border: 1px solid rgba(56, 189, 248, 0.25);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    margin-top: 0.2rem;
  }

  .surface-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--hud-cyan, #38bdf8);
    display: block;
    margin-bottom: 0.2rem;
  }

  .surface-coords {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    font-size: 0.75rem;
    color: var(--hud-text);
  }

  .font-mono {
    font-family: var(--font-mono);
  }
</style>
