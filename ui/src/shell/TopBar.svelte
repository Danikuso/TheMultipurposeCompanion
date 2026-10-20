<script lang="ts">
  import { credits, pct } from '../lib/format';
  import { t } from '../lib/i18n';
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
  </div>

  <div class="topbar-metrics">
    <div class="metric">
      <span class="metric-label">{t('field-system')}</span>
      <strong class="metric-value" data-testid="topbar-system">{systemText}</strong>
    </div>
    <div class="metric">
      <span class="metric-value">{t(modeKey)}</span>
    </div>
    <div class="metric">
      <span class="metric-label">{t('field-fuel')}</span>
      <span class="metric-value">{pct(fuelRatio)}</span>
    </div>
    <div class="metric">
      <span class="metric-label">{t('field-hull')}</span>
      <span class="metric-value">{pct(ship?.hull_health)}</span>
    </div>
    <div class="metric">
      <span class="metric-label">{t('field-cargo')}</span>
      <span class="metric-value">{cargoText}</span>
    </div>
    <div class="metric">
      <span class="metric-label">{t('field-credits')}</span>
      <span class="metric-value">{credits(cmdr?.credits)}</span>
    </div>
  </div>

  <div class="topbar-status">
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
    border-bottom: 1px solid var(--hud-border);
    padding: 0.5rem 1rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .topbar-brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 700;
    color: var(--hud-primary);
  }

  .conn-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--hud-danger);
    display: inline-block;
  }

  .conn-dot.online {
    background: var(--hud-success);
  }

  .topbar-metrics {
    display: flex;
    align-items: center;
    gap: 1.25rem;
    flex-wrap: wrap;
  }

  .metric {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
  }

  .metric-label {
    color: var(--hud-muted);
    font-size: 0.8rem;
  }

  .metric-value {
    font-family: var(--font-mono);
    font-weight: 600;
  }

  .topbar-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border);
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
