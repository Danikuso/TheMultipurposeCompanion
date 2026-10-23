<script lang="ts">
  import { credits, pct } from '../lib/format';
  import { t } from '../lib/i18n';
  import { prefs, setAutoCompanion } from '../lib/prefs.svelte';
  import { live, topic } from '../lib/ws.svelte';
  import { diag, diagOn } from '../lib/diag.svelte';

  interface LocationTopic {
    system?: string;
    mode?: string;
  }

  interface ShipTopic {
    fuel_main?: number | null;
    fuel_capacity_main?: number | null;
    hull_health?: number | null;
    cargo_count?: number | null;
    cargo_capacity?: number | null;
  }

  interface CmdrTopic {
    credits?: number | null;
  }

  interface GameTopic {
    running?: boolean;
    galaxy?: string;
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
  const ship = $derived(topic<ShipTopic>('core.ship'));
  const cmdr = $derived(topic<CmdrTopic>('core.cmdr'));
  const game = $derived(topic<GameTopic>('core.game'));

  const systemText = $derived(loc?.system && loc.system.length > 0 ? loc.system : '—');
  const modeKey = $derived(MODE_KEY_MAP[loc?.mode ?? 'Unknown'] ?? 'mode-unknown');
  const fuelRatio = $derived(
    ship?.fuel_main != null && ship?.fuel_capacity_main && ship.fuel_capacity_main > 0
      ? ship.fuel_main / ship.fuel_capacity_main
      : null,
  );
  const cargoText = $derived(
    ship?.cargo_count != null && ship?.cargo_capacity != null
      ? `${ship.cargo_count} / ${ship.cargo_capacity}`
      : ship?.cargo_count != null
        ? String(ship.cargo_count)
        : '—',
  );

  $effect(() => {
    if (!diagOn()) return;
    diag('ui.topbar.state', {
      connected: live.connected,
      has_location: loc !== undefined,
      has_ship: ship !== undefined,
      has_cmdr: cmdr !== undefined,
      game_running: game?.running === true,
      galaxy: game?.galaxy === 'Legacy' ? 'Legacy' : 'Live',
      mode: loc?.mode ?? 'Unknown',
      fuel_ratio: fuelRatio === null ? -1 : Math.round(fuelRatio * 100) / 100,
    });
  });
</script>

<header class="hud-topbar">
  <div class="topbar-brand">
    <span class="conn-dot" class:online={live.connected}></span>
    <span class="brand-title">{t('app-name')}</span>
    {#if game?.running}
      <span class="badge badge-cyan" data-testid="topbar-galaxy">{game?.galaxy === 'Legacy' ? 'LEGACY' : 'LIVE 4.0'}</span>
    {/if}
  </div>

  <div class="topbar-metrics">
    <div class="metric-chip">
      <span class="metric-chip-label">{t('field-system')}</span>
      <strong class="metric-chip-val" data-testid="topbar-system">{systemText}</strong>
    </div>
    <div class="metric-chip">
      <span class="metric-chip-label">MODO</span>
      <span class="metric-chip-val">{t(modeKey)}</span>
    </div>
    <div class="metric-chip bar-chip">
      <div class="bar-chip-header">
        <span class="metric-chip-label">{t('field-fuel')}</span>
        <span class="metric-chip-val font-mono" class:val-warn={fuelRatio != null && fuelRatio < 0.25}>{pct(fuelRatio)}</span>
      </div>
      {#if fuelRatio != null}
        <div class="hud-meter-track mini-meter">
          <div class="hud-meter-fill fill-green" class:fill-orange={fuelRatio < 0.25} style="width: {Math.max(0, Math.min(100, fuelRatio * 100))}%;"></div>
        </div>
      {/if}
    </div>
    <div class="metric-chip bar-chip">
      <div class="bar-chip-header">
        <span class="metric-chip-label">{t('field-hull')}</span>
        <span class="metric-chip-val font-mono" class:val-danger={ship?.hull_health != null && ship.hull_health < 0.3}>{pct(ship?.hull_health)}</span>
      </div>
      {#if ship?.hull_health != null}
        <div class="hud-meter-track mini-meter">
          <div class="hud-meter-fill fill-cyan" class:fill-danger={ship.hull_health < 0.3} style="width: {Math.max(0, Math.min(100, ship.hull_health * 100))}%;"></div>
        </div>
      {/if}
    </div>
    <div class="metric-chip">
      <span class="metric-chip-label">{t('field-cargo')}</span>
      <span class="metric-chip-val font-mono">{cargoText}</span>
    </div>
    <div class="metric-chip">
      <span class="metric-chip-label">{t('field-credits')}</span>
      <span class="metric-chip-val font-mono val-gold">{credits(cmdr?.credits)}</span>
    </div>
  </div>

  <div class="topbar-status">
    <label class="auto-switch" title={t('auto-companion-tooltip')}>
      <input type="checkbox" role="switch" data-testid="auto-companion" checked={prefs.autoCompanion}
        onchange={(e) => { void setAutoCompanion(e.currentTarget.checked); }} />
      <span class="switch-capsule" class:on={prefs.autoCompanion}>
        <span class="capsule-led"></span>
        <span class="capsule-text">{t('auto-companion')}</span>
      </span>
    </label>
    {#if game?.galaxy === 'Legacy'}
      <span class="badge badge-warning">{t('galaxy-legacy-warning')}</span>
    {/if}
    {#if !game?.running}
      <span class="badge badge-muted">{t('game-not-running')}</span>
    {/if}
  </div>
</header>

<style>
  .hud-topbar {
    position: sticky;
    top: 0;
    z-index: 20;
    background: var(--hud-surface);
    border-bottom: 1px solid var(--hud-border-subtle);
    padding: 0.5rem 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    box-shadow: 0 2px 15px rgba(0, 0, 0, 0.4);
  }

  .topbar-brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-weight: 700;
    color: var(--hud-primary);
    letter-spacing: 0.05em;
  }

  .conn-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--hud-danger);
    display: inline-block;
    transition: background-color 150ms ease, box-shadow 150ms ease;
  }

  .conn-dot.online {
    background: var(--hud-success);
    box-shadow: 0 0 8px var(--hud-success);
  }

  .topbar-metrics {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .metric-chip {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.35rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 80px;
  }

  .bar-chip { min-width: 110px; }
  .bar-chip-header { display: flex; justify-content: space-between; align-items: baseline; }

  .metric-chip-label {
    color: var(--hud-muted);
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metric-chip-val {
    font-size: 0.88rem;
    color: var(--hud-text);
  }

  .font-mono { font-family: var(--font-mono); }
  .val-gold { color: #fbbf24; font-weight: 700; }
  .val-warn { color: var(--hud-warning); font-weight: 700; }
  .val-danger { color: var(--hud-danger); font-weight: 700; }

  .mini-meter { height: 5px; margin-top: 3px; }

  .topbar-status {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .auto-switch {
    display: flex;
    align-items: center;
    cursor: pointer;
  }

  .auto-switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .switch-capsule {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 20px;
    padding: 0.3rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--hud-muted);
    transition: all 150ms ease;
  }

  .switch-capsule.on {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.4);
    color: var(--hud-success);
  }

  .capsule-led {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--hud-muted);
    transition: background-color 150ms ease, box-shadow 150ms ease;
  }

  .switch-capsule.on .capsule-led {
    background: var(--hud-success);
    box-shadow: 0 0 6px var(--hud-success);
  }

  .badge-warning {
    background: rgba(245, 158, 11, 0.2);
    border-color: var(--hud-warning);
    color: var(--hud-warning);
  }

  .badge-muted {
    color: var(--hud-muted);
  }
</style>
