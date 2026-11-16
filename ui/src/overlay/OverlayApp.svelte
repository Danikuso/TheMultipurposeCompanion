<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { credits, cssPct, decimal, integer, meters, pct, starClassLabel } from '../lib/format';
  import { connect, topic } from '../lib/ws.svelte';
  import {
    loadPrefs,
    prefs,
    toggleOverlayLock,
    updateOverlaySettings,
    type OverlayAnchor,
  } from '../lib/prefs.svelte';
  import { materialName, namesFrom, type ProspectorTopic, type MiningSessionTopic, type SellTopic } from '../modules/mining/types';
  import type { SystemTopic as ExplorationSystemTopic, UnsoldTopic as ExplorationUnsoldTopic } from '../modules/exploration/types';
  import type { BodyTopic as ExoBodyTopic, SamplingTopic, UnsoldTopic as ExoUnsoldTopic } from '../modules/exobiology/types';
  import type { RouteTopic } from '../modules/navigation/types';
  import type { ShipTopic as CombatShipTopic, SessionTopic as CombatSessionTopic, TargetTopic as CombatTargetTopic } from '../modules/combat/types';
  import { commodityName, type CargoTopic, type RouteTopic as TradeRouteTopic } from '../modules/trade/types';
  import type { ColonisationDepotTopic } from '../modules/colonisation/types';
  import { distance3dLy, resolveOverlayRole } from './overlayModel';

  interface LocationTopic {
    system?: string;
    star_pos?: [number, number, number];
    body?: string | null;
  }

  interface CoreShipTopic {
    fuel_main?: number | null;
    fuel_capacity_main?: number | null;
    hull_health?: number | null;
    cargo_count?: number | null;
    cargo_capacity?: number | null;
  }

  let { autoConnect = true }: { autoConnect?: boolean } = $props();

  let draggedManually = $state(false);

  onMount(() => {
    if (!autoConnect) return;
    connect();
    fetch('/api/config')
      .then((res) => (res.ok ? res.json() : null))
      .then((cfg) => {
        if (cfg) loadPrefs(cfg);
      })
      .catch(() => {});
  });

  const liveModules = $derived(topic<{ id: string; relevance: string }[]>('core.modules') ?? []);
  const loc = $derived(topic<LocationTopic>('core.location'));
  const ship = $derived(topic<CoreShipTopic>('core.ship'));
  const navRoute = $derived(topic<RouteTopic>('navigation.route'));
  const expSystem = $derived(topic<ExplorationSystemTopic>('exploration.system'));
  const expUnsold = $derived(topic<ExplorationUnsoldTopic>('exploration.unsold'));
  const exoBody = $derived(topic<ExoBodyTopic>('exobiology.body'));
  const exoSampling = $derived(topic<SamplingTopic>('exobiology.sampling'));
  const exoUnsold = $derived(topic<ExoUnsoldTopic>('exobiology.unsold'));
  const combatTarget = $derived(topic<CombatTargetTopic>('combat.target'));
  const combatShip = $derived(topic<CombatShipTopic>('combat.ship'));
  const combatSession = $derived(topic<CombatSessionTopic>('combat.session'));
  const miningProspector = $derived(topic<ProspectorTopic>('mining.prospector'));
  const miningSession = $derived(topic<MiningSessionTopic>('mining.session'));
  const miningSell = $derived(topic<SellTopic>('mining.sell'));
  const tradeCargo = $derived(topic<CargoTopic>('trade.cargo'));
  const tradeRoute = $derived(topic<TradeRouteTopic>('trade.route'));
  const colDepot = $derived(topic<ColonisationDepotTopic>('colonisation.depot'));

  const settings = $derived(prefs.overlay);

  const currentSystem = $derived(navRoute?.current?.name || loc?.system || '—');
  const currentScoopable = $derived(navRoute?.current?.scoopable === true);
  const nextHop = $derived(navRoute?.next?.name ?? null);
  const jumpsRemaining = $derived(navRoute?.remaining_jumps ?? null);

  const fuelRatio = $derived(
    ship?.fuel_main != null && ship?.fuel_capacity_main && ship.fuel_capacity_main > 0
      ? Math.max(0, Math.min(1, ship.fuel_main / ship.fuel_capacity_main))
      : null,
  );
  const hullRatio = $derived(
    ship?.hull_health != null ? Math.max(0, Math.min(1, ship.hull_health)) : null,
  );
  const shieldsDown = $derived(combatShip?.shields_up === false);

  const activeRole = $derived(
    resolveOverlayRole(
      settings,
      {
        modules: liveModules,
        hasSamplingOrBio: Boolean(exoSampling || (exoBody && exoBody.bio_signals > 0)),
        hasCombatActivity: Boolean(combatTarget || combatShip?.alerts?.length || (combatSession?.kills ?? 0) > 0),
        hasProspector: Boolean(miningProspector?.current),
        hasColonisation: Boolean(colDepot?.active_project),
        hasTrade: Boolean((tradeCargo?.used ?? 0) > 0 || (tradeRoute?.plan?.stops?.length ?? 0) > 0),
        hasExploration: Boolean(expSystem && expSystem.rows?.length > 0),
      },
      prefs.autoCompanionPriorities,
    ),
  );

  // Exobiología derivada
  const exoDistWidth = $derived.by(() => {
    if (!exoSampling || exoSampling.distance_m == null || !(exoSampling.min_m > 0)) return '0%';
    return cssPct((exoSampling.distance_m / exoSampling.min_m) * 100);
  });
  const exoTopCandidate = $derived(exoBody?.candidates?.find((c) => c.status !== 'done') ?? exoBody?.candidates?.[0] ?? null);

  // Exploración derivada
  const expProgress = $derived(expSystem ? (expSystem.fss?.complete ? 100 : Math.round((expSystem.fss?.progress ?? 0) * 100)) : 0);
  const expTopBody = $derived.by(() => {
    const rows = expSystem?.rows ?? [];
    return (
      rows.find((r) => (r.advice ?? []).includes('map')) ??
      rows.find((r) => (r.bio ?? 0) > 0) ??
      rows[0] ??
      null
    );
  });

  // Combate derivada
  const combatVouchers = $derived(
    combatSession ? (combatSession.vouchers_total?.bounty ?? 0) + (combatSession.vouchers_total?.combat_bond ?? 0) : 0,
  );

  // Minería derivada
  const miningNames = $derived(namesFrom(miningProspector, miningSession?.names));
  const miningTopMat = $derived.by(() => {
    const mats = miningProspector?.current?.materials ?? [];
    return mats.find((m) => m.interest) ?? mats[0] ?? null;
  });

  // Comercio derivada
  const tradeTopLine = $derived(tradeCargo?.lines?.[0] ?? null);
  const tradeCurrentStop = $derived(
    tradeRoute?.plan?.stops?.[tradeRoute.current ?? 0] ?? tradeRoute?.plan?.stops?.[0] ?? null,
  );
  const tradeDest = $derived(
    tradeCurrentStop?.to_system ?? tradeRoute?.copy_target ?? tradeTopLine?.best_nearby?.system_name ?? '—',
  );

  // Colonización derivada
  const colNextResource = $derived(colDepot?.resources?.find((r) => r.remaining > 0) ?? colDepot?.resources?.[0] ?? null);

  // Navegación / base derivada
  const homeDistance = $derived(distance3dLy(loc?.star_pos, prefs.homeSystemCoords));

  function startDrag() {
    draggedManually = true;
    fetch('/api/actions/overlay.start_drag', { method: 'POST' }).catch(() => {});
  }

  function handleLock() {
    if (draggedManually && typeof window !== 'undefined' && typeof window.screenX === 'number') {
      const dpr = window.devicePixelRatio || 1;
      const customX = Math.round(window.screenX * dpr);
      const customY = Math.round(window.screenY * dpr);
      draggedManually = false;
      void toggleOverlayLock({
        click_through: true,
        anchor: 'custom',
        custom_x: customX,
        custom_y: customY,
      });
      return;
    }
    draggedManually = false;
    void toggleOverlayLock({ click_through: true });
  }

  function selectAnchor(anchor: OverlayAnchor) {
    draggedManually = false;
    void updateOverlaySettings({ anchor });
  }

  const ANCHORS: { id: OverlayAnchor; icon: string; key: string }[] = [
    { id: 'top_left', icon: '↖', key: 'overlay-anchor-top-left' },
    { id: 'top_center', icon: '⬆', key: 'overlay-anchor-top-center' },
    { id: 'top_right', icon: '↗', key: 'overlay-anchor-top-right' },
    { id: 'bottom_left', icon: '↙', key: 'overlay-anchor-bottom-left' },
    { id: 'bottom_right', icon: '↘', key: 'overlay-anchor-bottom-right' },
  ];
</script>

<div
  class="overlay-shell"
  class:is-minimal={settings.mode === 'minimal'}
  class:is-tactical={settings.mode === 'tactical'}
  class:is-unlocked={!settings.click_through}
  data-testid="overlay-shell"
>
  {#if !settings.click_through}
    <div class="overlay-dragbar" data-testid="overlay-dragbar">
      <button
        type="button"
        class="drag-handle"
        data-testid="overlay-drag-handle"
        onmousedown={startDrag}
        title={t('overlay-header-drag')}
      >
        <span aria-hidden="true">⋮⋮</span>
        <span>{t('overlay-header-drag')}</span>
      </button>

      <div class="anchor-strip" role="group" aria-label={t('settings-overlay-anchor')}>
        {#each ANCHORS as a (a.id)}
          <button
            type="button"
            class="anchor-btn"
            class:active={settings.anchor === a.id}
            data-testid={`overlay-anchor-${a.id}`}
            title={t(a.key)}
            onclick={() => selectAnchor(a.id)}
          >{a.icon}</button>
        {/each}
      </div>

      <button
        type="button"
        class="mode-btn"
        data-testid="overlay-drag-mode-toggle"
        onclick={() => void updateOverlaySettings({ mode: settings.mode === 'minimal' ? 'tactical' : 'minimal' })}
      >
        {settings.mode === 'minimal' ? t('overlay-mode-tactical-short') : t('overlay-mode-minimal-short')}
      </button>

      <button
        type="button"
        class="pin-btn"
        data-testid="overlay-lock-btn"
        title={t('overlay-pin-tooltip')}
        onclick={handleLock}
      >
        🔒 {t('overlay-pin-btn')}
      </button>
    </div>
  {/if}

  <!-- Tira fija superior (visible siempre en Minimal y como cabecera en Tactical) -->
  <div class="ov-strip" data-testid="overlay-strip">
    <div class="ov-sys">
      <span class="ov-dot" class:scoop={currentScoopable} title={currentScoopable ? t('overlay-scoopable') : ''}></span>
      <span class="ov-sys-name" data-testid="overlay-system">{currentSystem}</span>
      {#if currentScoopable}
        <span class="ov-tag tag-green" data-testid="overlay-scoopable" title={t('overlay-scoopable')}>⛽</span>
      {/if}
    </div>

    {#if nextHop}
      <div class="ov-hop" data-testid="overlay-next-hop">
        <span class="ov-arrow">→</span>
        <span class="ov-hop-name">{nextHop}</span>
        {#if jumpsRemaining != null && jumpsRemaining > 0}
          <span class="ov-jumps">{t('overlay-jumps-left', { count: jumpsRemaining })}</span>
        {/if}
      </div>
    {/if}

    <div class="ov-vitals">
      {#if shieldsDown}
        <span class="ov-chip danger" data-testid="overlay-shield-alert">🛡️ {t('overlay-shield-down')}</span>
      {/if}
      {#if hullRatio != null && hullRatio < 0.995}
        <span
          class="ov-chip"
          class:danger={hullRatio < 0.35}
          class:warn={hullRatio >= 0.35}
          data-testid="overlay-hull-chip"
        >{t('overlay-hull-alert')} {pct(hullRatio)}</span>
      {/if}
      <span
        class="ov-chip"
        class:warn={fuelRatio != null && fuelRatio < 0.25}
        class:ok={fuelRatio != null && fuelRatio >= 0.25}
        data-testid="overlay-fuel-chip"
      >⛽ {pct(fuelRatio)}</span>
    </div>
  </div>

  <!-- Bloque táctico adaptativo (solo en modo Tactical y si hay al menos un bloque activo) -->
  {#if settings.mode === 'tactical' && activeRole}
    <div class="ov-body" data-testid="overlay-tactical-body" data-role={activeRole}>
      {#if activeRole === 'exobiology'}
        <div class="ov-role-row">
          <span class="ov-badge badge-purple">{t('overlay-role-exobiology')}</span>
          <span class="ov-mono ov-muted">{exoBody?.body ?? loc?.body ?? '—'}</span>
        </div>
        {#if exoSampling}
          <div class="ov-main-row">
            <strong class="ov-ellipsis">{exoSampling.species_name ?? exoSampling.genus_name ?? '—'}</strong>
            <span class="ov-chip ok">{t('overlay-exo-samples', { points: exoSampling.points })}</span>
          </div>
          {#if !exoSampling.complete}
            <div class="ov-meter-track">
              <div
                class="ov-meter-fill"
                class:complete={exoSampling.remaining_m === 0}
                style="width: {exoDistWidth};"
              ></div>
            </div>
            <div class="ov-sub-row ov-mono">
              <span>{meters(exoSampling.distance_m)} / {meters(exoSampling.min_m)}</span>
              {#if exoSampling.direction}
                <span>🧭 {t(`compass-${exoSampling.direction.compass}`)}</span>
              {/if}
              {#if exoUnsold}
                <span class="ov-gold">{credits(exoUnsold.total)}</span>
              {/if}
            </div>
          {/if}
        {:else if exoBody}
          <div class="ov-main-row">
            <span class="ov-ellipsis">{t('overlay-exo-no-species', { signals: exoBody.bio_signals })}</span>
            {#if exoTopCandidate}
              <span class="ov-gold">{credits(exoBody.first_logged ? exoTopCandidate.value_x5 : exoTopCandidate.value)}</span>
            {/if}
          </div>
          {#if exoTopCandidate}
            <div class="ov-sub-row">
              <span class="ov-ellipsis">{exoTopCandidate.species_name ?? exoTopCandidate.genus_name ?? '—'}</span>
              <span class="ov-mono ov-muted">{meters(exoTopCandidate.min_m)}</span>
            </div>
          {/if}
        {:else}
          <p class="ov-empty">{t('overlay-no-active-data')}</p>
        {/if}
      {:else if activeRole === 'exploration'}
        <div class="ov-role-row">
          <span class="ov-badge badge-cyan">{t('overlay-role-exploration')}</span>
          {#if expSystem?.star_classes?.length}
            <span class="ov-mono ov-muted">{starClassLabel(expSystem.star_classes[0])}</span>
          {/if}
        </div>
        {#if expSystem}
          <div class="ov-main-row">
            <span>{t('overlay-exp-fss', { progress: `${expProgress}%`, targets: expSystem.map_targets ?? 0 })}</span>
            {#if expUnsold}
              <span class="ov-gold ov-mono">{credits(expUnsold.total)}</span>
            {/if}
          </div>
          <div class="ov-sub-row">
            <span>{t('overlay-exp-signals', { bio: expSystem.bio_signals ?? 0, tf: expSystem.terraformable_count ?? 0 })}</span>
            {#if expTopBody}
              <span class="ov-mono ov-cyan ov-ellipsis">{expTopBody.short_name}</span>
            {/if}
          </div>
        {:else}
          <p class="ov-empty">{t('overlay-no-active-data')}</p>
        {/if}
      {:else if activeRole === 'combat'}
        <div class="ov-role-row">
          <span class="ov-badge badge-danger">{t('overlay-role-combat')}</span>
          {#if combatTarget?.badge?.kind === 'wanted'}
            <span class="ov-chip danger ov-mono">{credits(combatTarget.badge.bounty)}</span>
          {/if}
        </div>
        {#if combatTarget}
          <div class="ov-main-row">
            <strong class="ov-ellipsis">
              {t('overlay-combat-target-hull', {
                target: combatTarget.ship_name ?? combatTarget.ship_key ?? '—',
                shield: pct(combatTarget.shield_pct != null ? combatTarget.shield_pct / 100 : null),
                hull: pct(combatTarget.hull_pct != null ? combatTarget.hull_pct / 100 : null),
              })}
            </strong>
          </div>
        {:else}
          <div class="ov-main-row">
            <span class="ov-muted">{t('overlay-combat-no-target')}</span>
          </div>
        {/if}
        <div class="ov-sub-row ov-mono">
          <span>{t('overlay-combat-vouchers', { credits: credits(combatVouchers), kills: combatSession?.kills ?? 0 })}</span>
        </div>
      {:else if activeRole === 'mining'}
        <div class="ov-role-row">
          <span class="ov-badge badge-orange">{t('overlay-role-mining')}</span>
          {#if miningProspector?.current?.motherlode}
            <span class="ov-chip warn ov-mono">{materialName(miningProspector.current.motherlode, miningNames)}</span>
          {/if}
        </div>
        {#if miningTopMat}
          <div class="ov-main-row">
            <strong class="ov-ellipsis">{materialName(miningTopMat.key, miningNames)}</strong>
            <span class="ov-chip ok ov-mono">{decimal(miningTopMat.proportion, 1)} %</span>
          </div>
        {:else}
          <div class="ov-main-row">
            <span class="ov-muted">{t('overlay-mining-no-rock')}</span>
          </div>
        {/if}
        <div class="ov-sub-row ov-mono">
          <span>
            {t('overlay-mining-cargo', {
              used: miningSession?.cargo ?? ship?.cargo_count ?? 0,
              cap: miningSession?.capacity ?? ship?.cargo_capacity ?? 0,
              limpets: miningSession?.limpets_left ?? 0,
            })}
          </span>
          {#if miningSell?.status === 'ok' && miningSell.best_station_total}
            <span class="ov-gold">{credits(miningSell.best_station_total[1])}</span>
          {/if}
        </div>
      {:else if activeRole === 'colonisation'}
        <div class="ov-role-row">
          <span class="ov-badge badge-gold">{t('overlay-role-colonisation')}</span>
          <span class="ov-mono ov-ellipsis ov-muted">{colDepot?.station_name ?? '—'}</span>
        </div>
        {#if colDepot?.active_project}
          <div class="ov-main-row">
            <span>{t('overlay-col-depot', { progress: `${Math.round(colDepot.progress_percent)} %`, trips: colDepot.total_trips_remaining })}</span>
          </div>
          <div class="ov-meter-track">
            <div class="ov-meter-fill" style="width: {cssPct(colDepot.progress_percent)};"></div>
          </div>
          {#if colNextResource}
            <div class="ov-sub-row ov-mono">
              <span class="ov-ellipsis">{colNextResource.name_localised ?? colNextResource.name}</span>
              <span class="ov-gold">-{integer(colNextResource.remaining)} t</span>
            </div>
          {/if}
        {:else}
          <p class="ov-empty">{t('overlay-no-active-data')}</p>
        {/if}
      {:else if activeRole === 'trade'}
        <div class="ov-role-row">
          <span class="ov-badge badge-green">{t('overlay-role-trade')}</span>
          {#if tradeTopLine}
            <span class="ov-mono ov-gold">{commodityName(tradeTopLine.commodity, tradeTopLine.name_localised)} ({tradeTopLine.tons} t)</span>
          {/if}
        </div>
        {#if (tradeCargo?.used ?? 0) > 0 || tradeCurrentStop}
          <div class="ov-main-row">
            <span class="ov-ellipsis">
              {t('overlay-trade-cargo', {
                used: tradeCargo?.used ?? ship?.cargo_count ?? 0,
                cap: tradeCargo?.capacity ?? ship?.cargo_capacity ?? 0,
                dest: tradeDest,
              })}
            </span>
          </div>
        {:else}
          <p class="ov-empty">{t('overlay-trade-no-cargo')}</p>
        {/if}
      {:else}
        <div class="ov-role-row">
          <span class="ov-badge badge-cyan">{t('overlay-role-navigation')}</span>
          {#if loc?.body}
            <span class="ov-mono ov-muted ov-ellipsis">{loc.body}</span>
          {/if}
        </div>
        <div class="ov-main-row">
          {#if nextHop}
            <span class="ov-ellipsis">{t('overlay-nav-route', { next: nextHop, jumps: t('overlay-jumps-left', { count: jumpsRemaining ?? 0 }) })}</span>
          {:else}
            <span class="ov-ellipsis">{t('overlay-nav-free', { base: prefs.homeSystem ?? t('overlay-nav-base-none') })}</span>
          {/if}
        </div>
        {#if homeDistance != null && prefs.homeSystem}
          <div class="ov-sub-row ov-mono">
            <span>{prefs.homeSystem}: {decimal(homeDistance, 1)} {t('unit-light-year')}</span>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0b0e16;
    overflow: hidden;
  }

  .overlay-shell {
    box-sizing: border-box;
    width: 100%;
    max-width: 436px;
    background: rgba(11, 14, 22, 0.96);
    border: 1px solid rgba(255, 113, 0, 0.48);
    border-radius: 6px;
    color: var(--hud-text, #f3f4f6);
    font-family: var(--font-sans, system-ui, sans-serif);
    font-size: 12px;
    padding: 0.45rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    user-select: none;
    overflow: hidden;
  }

  .overlay-shell.is-tactical {
    max-width: 416px;
  }

  .overlay-shell.is-unlocked {
    border-color: #f59e0b;
    box-shadow: inset 0 0 0 1px rgba(245, 158, 11, 0.35);
  }

  .overlay-dragbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem;
    padding-bottom: 0.35rem;
    border-bottom: 1px solid rgba(245, 158, 11, 0.35);
  }

  .drag-handle {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: rgba(245, 158, 11, 0.14);
    border: 1px solid rgba(245, 158, 11, 0.45);
    color: #fbbf24;
    border-radius: 4px;
    padding: 0.15rem 0.45rem;
    font-size: 11px;
    font-weight: 700;
    cursor: grab;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .anchor-strip {
    display: inline-flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .anchor-btn,
  .mode-btn,
  .pin-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: var(--hud-text, #f3f4f6);
    border-radius: 4px;
    font-size: 11px;
    padding: 0.15rem 0.35rem;
    cursor: pointer;
    line-height: 1.2;
    flex-shrink: 0;
  }

  .anchor-btn.active {
    background: rgba(255, 113, 0, 0.22);
    border-color: var(--hud-primary, #ff7100);
    color: #fff;
  }

  .pin-btn {
    background: rgba(34, 197, 94, 0.16);
    border-color: rgba(34, 197, 94, 0.45);
    color: #4ade80;
    font-weight: 700;
  }

  .ov-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.45rem;
    min-height: 26px;
  }

  .ov-sys {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    flex: 1 1 auto;
  }

  .ov-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--hud-primary, #ff7100);
    flex-shrink: 0;
  }

  .ov-dot.scoop {
    background: #22c55e;
  }

  .ov-sys-name {
    font-weight: 700;
    font-size: 12.5px;
    color: #fff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ov-tag {
    font-size: 11px;
    flex-shrink: 0;
  }

  .ov-hop {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    min-width: 0;
    flex: 0 1 155px;
    color: var(--hud-cyan, #06b6d4);
    font-weight: 600;
  }

  .ov-hop-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ov-jumps {
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    background: rgba(6, 182, 212, 0.14);
    border: 1px solid rgba(6, 182, 212, 0.35);
    border-radius: 3px;
    padding: 0 4px;
    flex-shrink: 0;
  }

  .ov-vitals {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
  }

  .ov-chip {
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.05);
    white-space: nowrap;
  }

  .ov-chip.ok {
    color: #4ade80;
    border-color: rgba(34, 197, 94, 0.35);
    background: rgba(34, 197, 94, 0.12);
  }

  .ov-chip.warn {
    color: #fbbf24;
    border-color: rgba(245, 158, 11, 0.45);
    background: rgba(245, 158, 11, 0.15);
  }

  .ov-chip.danger {
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.55);
    background: rgba(239, 68, 68, 0.2);
  }

  .ov-body {
    border-top: 1px solid rgba(255, 255, 255, 0.09);
    padding-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .ov-role-row,
  .ov-main-row,
  .ov-sub-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    min-width: 0;
  }

  .ov-badge {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    border: 1px solid transparent;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-purple { color: #d8b4fe; background: rgba(168, 85, 247, 0.16); border-color: rgba(168, 85, 247, 0.4); }
  .badge-cyan { color: #67e8f9; background: rgba(6, 182, 212, 0.15); border-color: rgba(6, 182, 212, 0.4); }
  .badge-danger { color: #fca5a5; background: rgba(239, 68, 68, 0.16); border-color: rgba(239, 68, 68, 0.4); }
  .badge-orange { color: #fdba74; background: rgba(255, 113, 0, 0.16); border-color: rgba(255, 113, 0, 0.4); }
  .badge-gold { color: #fde68a; background: rgba(245, 158, 11, 0.16); border-color: rgba(245, 158, 11, 0.4); }
  .badge-green { color: #86efac; background: rgba(34, 197, 94, 0.16); border-color: rgba(34, 197, 94, 0.4); }

  .ov-meter-track {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.1);
    overflow: hidden;
  }

  .ov-meter-fill {
    height: 100%;
    background: var(--hud-primary, #ff7100);
  }

  .ov-meter-fill.complete {
    background: var(--hud-success, #22c55e);
  }

  .ov-ellipsis {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .ov-mono { font-family: var(--font-mono, monospace); font-size: 11px; }
  .ov-muted { color: var(--hud-muted, #9ca3af); }
  .ov-gold { color: #fbbf24; font-weight: 700; white-space: nowrap; }
  .ov-cyan { color: #38bdf8; }
  .ov-empty { margin: 0.25rem 0; color: var(--hud-muted, #9ca3af); font-size: 11.5px; }
</style>
