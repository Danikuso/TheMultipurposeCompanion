<script lang="ts">
  import { credits, ly, pct, integer, formatShipName } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import GradeBadge from '../../lib/icons/GradeBadge.svelte';
  import {
    categorizeSlot,
    formatSlotName,
    formatItemName,
    type FormattedModule,
  } from './shipFormatters';

  interface LoadoutModuleData {
    Slot?: string;
    Item?: string;
    On?: boolean;
    Priority?: number;
    Health?: number;
    AmmoInClip?: number;
    AmmoInHopper?: number;
    Engineering?: {
      BlueprintName?: string;
      Level?: number;
      Engineer?: string;
      ExperimentalEffect?: string;
      ExperimentalEffect_Localised?: string;
      Modifiers?: {
        Label?: string;
        Value?: number;
        OriginalValue?: number;
        LessIsGood?: number;
      }[];
    };
  }

  interface ShipTopic {
    ship_type?: string;
    ship_type_localised?: string | null;
    name?: string | null;
    ident?: string | null;
    hull_health?: number | null;
    fuel_main?: number | null;
    fuel_reservoir?: number | null;
    fuel_capacity_main?: number | null;
    fuel_capacity_reserve?: number | null;
    cargo_count?: number | null;
    cargo_capacity?: number | null;
    rebuy?: number | null;
    max_jump_range?: number | null;
    unladen_mass?: number | null;
    damaged_modules?: string[];
    modules?: LoadoutModuleData[];
    cargo?: { Name: string; Name_Localised?: string | null; Count: number }[];
  }

  interface CombatShipTopic {
    shields_up?: boolean | null;
    hull_pct?: number | null;
    heat?: string;
    pips?: [number, number, number];
  }

  const ship = $derived(topic<ShipTopic>('core.ship'));
  const combatShip = $derived(topic<CombatShipTopic>('combat.ship'));

  // Porcentajes de combustible para barras de progreso
  const fuelMainPct = $derived(
    ship?.fuel_main != null && ship?.fuel_capacity_main && ship.fuel_capacity_main > 0
      ? Math.min(100, Math.max(0, Math.round((ship.fuel_main / ship.fuel_capacity_main) * 100)))
      : 0,
  );
  const fuelResPct = $derived(
    ship?.fuel_reservoir != null && ship?.fuel_capacity_reserve && ship.fuel_capacity_reserve > 0
      ? Math.min(100, Math.max(0, Math.round((ship.fuel_reservoir / ship.fuel_capacity_reserve) * 100)))
      : 0,
  );
  const cargoPct = $derived(
    ship?.cargo_count != null && ship?.cargo_capacity && ship.cargo_capacity > 0
      ? Math.min(100, Math.max(0, Math.round((ship.cargo_count / ship.cargo_capacity) * 100)))
      : 0,
  );

  // Módulos parseados y formateados
  const formattedModules = $derived<FormattedModule[]>(
    (ship?.modules ?? []).map((m) => {
      const slot = m.Slot ?? '';
      const rawItem = m.Item ?? '';
      const category = categorizeSlot(slot);
      const slotLabel = formatSlotName(slot);
      const { label: itemLabel, sizeClass } = formatItemName(rawItem);
      const healthPct = m.Health != null ? Math.round(m.Health * 100) : 100;
      const on = m.On ?? true;
      const priority = (m.Priority ?? 0) + 1; // 1-based para UI
      const ammo =
        m.AmmoInClip != null && m.AmmoInHopper != null
          ? `${m.AmmoInClip} / ${m.AmmoInHopper}`
          : m.AmmoInClip != null
            ? String(m.AmmoInClip)
            : null;

      const eng = m.Engineering
        ? {
            blueprint: m.Engineering.BlueprintName?.replace(/^([A-Za-z]+_)/, '')?.replace(/_/g, ' ') ?? 'Mod',
            level: m.Engineering.Level ?? 1,
            engineer: m.Engineering.Engineer,
            experimental: m.Engineering.ExperimentalEffect_Localised || m.Engineering.ExperimentalEffect?.replace(/_/g, ' '),
            modifiers: (m.Engineering.Modifiers ?? []).map((mod) => ({
              label: mod.Label ?? '',
              value: mod.Value ?? null,
              original: mod.OriginalValue ?? null,
              lessIsGood: mod.LessIsGood === 1,
            })),
          }
        : null;

      return {
        slot,
        slotLabel,
        category,
        item: rawItem,
        itemLabel,
        sizeClass,
        healthPct,
        on,
        priority,
        ammo,
        engineering: eng,
      };
    }),
  );

  // Pestaña de estadísticas seleccionada ('exploration' | 'combat' | 'cargo')
  let activeTab = $state<'exploration' | 'combat' | 'cargo'>('exploration');

  // Filtro de categoría de módulos ('all' | 'hardpoint' | 'core' | 'optional' | 'utility')
  let categoryFilter = $state<'all' | 'hardpoint' | 'core' | 'optional' | 'utility'>('all');
  let searchQuery = $state('');
  let modulesCollapsed = $state(false);
  let expandedModuleSlot = $state<string | null>(null);

  function toggleModuleDetails(slot: string) {
    expandedModuleSlot = expandedModuleSlot === slot ? null : slot;
  }

  // Filtrado de módulos
  const filteredModules = $derived(
    formattedModules.filter((m) => {
      if (categoryFilter !== 'all' && m.category !== categoryFilter) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        return (
          m.itemLabel.toLowerCase().includes(q) ||
          m.slotLabel.toLowerCase().includes(q) ||
          m.sizeClass.toLowerCase().includes(q) ||
          (m.engineering?.blueprint.toLowerCase().includes(q) ?? false)
        );
      }
      return true;
    }),
  );

  // Detección de módulos especializados para estadísticas
  const fuelScoop = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('fuelscoop')));
  const guardianBooster = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('guardianfsdbooster')));
  const dss = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('detailedsurfacescanner')));
  const srvBay = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('buggybay')));
  const afmu = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('repairer')));
  const refinery = $derived(formattedModules.find((m) => m.item.toLowerCase().includes('refinery')));

  // Armas y utilitarios
  const hardpoints = $derived(formattedModules.filter((m) => m.category === 'hardpoint'));
  const utilities = $derived(formattedModules.filter((m) => m.category === 'utility'));
  const shieldBoosters = $derived(utilities.filter((m) => m.item.toLowerCase().includes('shieldbooster')));
  const chaff = $derived(utilities.filter((m) => m.item.toLowerCase().includes('chaff')));
  const heatSinks = $derived(utilities.filter((m) => m.item.toLowerCase().includes('heatsink')));
  const pointDefence = $derived(utilities.filter((m) => m.item.toLowerCase().includes('pointdefence')));

  // Bodegas de carga instaladas
  const cargoRacks = $derived(formattedModules.filter((m) => m.item.toLowerCase().includes('cargorack')));
  const limpets = $derived(formattedModules.filter((m) => m.item.toLowerCase().includes('dronecontrol')));
</script>

<Panel titleKey="panel-ship" span={2}>
  {#if !ship || (!ship.ship_type && ship.hull_health == null)}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="ship-container">
      <!-- 1. CABECERA Y METRICAS PRINCIPALES DE CABINA -->
      <div class="hud-card cockpit-header">
        <div class="identity-col">
          <div class="ship-titles">
            <h2 class="ship-type">{formatShipName(ship.ship_type, ship.ship_type_localised)}</h2>
            {#if ship.name || ship.ident}
              <span class="ship-name">
                {ship.name ?? ''}
                {#if ship.ident}<span class="ship-ident">[{ship.ident}]</span>{/if}
              </span>
            {/if}
          </div>
          <div class="badge-row">
            {#if ship.unladen_mass != null}
              <span class="hud-pill" title={t('ship-unladen-mass')}>
                ⚖️ {integer(ship.unladen_mass)} t
              </span>
            {/if}
            {#if ship.rebuy != null}
              <span class="hud-pill" title={t('field-rebuy')}>
                🛡️ {credits(ship.rebuy)}
              </span>
            {/if}
          </div>
        </div>

        <!-- 4 Medidores de Cabina -->
        <div class="gauges-grid">
          <!-- Casco -->
          <div class="gauge-item">
            <div class="gauge-meta">
              <span class="gauge-label">{t('field-hull')}</span>
              <strong class="gauge-val mono">{pct(ship.hull_health)}</strong>
            </div>
            <div class="meter-track">
              <div
                class="meter-fill"
                class:meter-fill--danger={(ship.hull_health ?? 1) < 0.4}
                class:meter-fill--warning={(ship.hull_health ?? 1) >= 0.4 && (ship.hull_health ?? 1) < 0.8}
                style="width: {Math.round((ship.hull_health ?? 1) * 100)}%"
              ></div>
            </div>
          </div>

          <!-- Escudos -->
          <div class="gauge-item">
            <div class="gauge-meta">
              <span class="gauge-label">{t('combat-shields')}</span>
              <span
                class="hud-pill mono"
                class:hud-pill--success={combatShip?.shields_up === true}
                class:hud-pill--danger={combatShip?.shields_up === false}
              >
                {combatShip?.shields_up == null ? '—' : combatShip.shields_up ? t('combat-shields-up') : t('combat-shields-down')}
              </span>
            </div>
            <div class="meter-track">
              <div
                class="meter-fill meter-fill--cyan"
                style="width: {combatShip?.shields_up ? 100 : 0}%"
              ></div>
            </div>
          </div>

          <!-- Combustible -->
          <div class="gauge-item">
            <div class="gauge-meta">
              <span class="gauge-label">{t('field-fuel')}</span>
              <span class="gauge-val mono">
                {ship.fuel_main?.toFixed(1) ?? '—'} / {ship.fuel_capacity_main?.toFixed(1) ?? '—'} t
              </span>
            </div>
            <div class="meter-track">
              <div
                class="meter-fill"
                data-testid="fuel-main-fill"
                style="width: {fuelMainPct}%"
              ></div>
            </div>
            {#if ship.fuel_reservoir != null}
              <div class="meter-track reserve-track" title="Reserva de combustible">
                <div class="meter-fill reserve-fill" style="width: {fuelResPct}%"></div>
              </div>
            {/if}
          </div>

          <!-- Carga -->
          <div class="gauge-item">
            <div class="gauge-meta">
              <span class="gauge-label">{t('field-cargo')}</span>
              <span class="gauge-val mono">{ship.cargo_count ?? 0} / {ship.cargo_capacity ?? 0} t</span>
            </div>
            <div class="meter-track">
              <div class="meter-fill meter-fill--gold" style="width: {cargoPct}%"></div>
            </div>
          </div>
        </div>
      </div>

      <!-- 2. PESTAÑAS DE ESTADISTICAS ESPECIALIZADAS (Exploración / Combate / Carga) -->
      <div class="hud-card stats-card">
        <div class="tab-buttons" role="tablist">
          <button
            type="button"
            class="tab-btn"
            class:active={activeTab === 'exploration'}
            onclick={() => activeTab = 'exploration'}
          >
            🧭 {t('ship-tab-exploration')}
          </button>
          <button
            type="button"
            class="tab-btn"
            class:active={activeTab === 'combat'}
            onclick={() => activeTab = 'combat'}
          >
            ⚔️ {t('ship-tab-combat')}
          </button>
          <button
            type="button"
            class="tab-btn"
            class:active={activeTab === 'cargo'}
            onclick={() => activeTab = 'cargo'}
          >
            📦 {t('ship-tab-cargo')}
          </button>
        </div>

        <div class="tab-content">
          {#if activeTab === 'exploration'}
            <div class="stat-columns">
              <div class="stat-col">
                <div class="metric-card">
                  <span class="metric-label">{t('field-jump-range')} (Max)</span>
                  <strong class="metric-val mono cyan">{ly(ship.max_jump_range)}</strong>
                </div>
                <div class="metric-card">
                  <span class="metric-label">{t('ship-unladen-mass')}</span>
                  <span class="metric-val mono">{ship.unladen_mass ? `${integer(ship.unladen_mass)} t` : '—'}</span>
                </div>
                <div class="metric-card">
                  <span class="metric-label">{t('field-fuel')} Total</span>
                  <span class="metric-val mono">{ship.fuel_capacity_main ?? 0} t</span>
                </div>
              </div>

              <div class="stat-col exploration-modules">
                <h4 class="sub-heading">{t('ship-exploration-equipment')}</h4>
                <div class="feature-pills">
                  <div class="feature-item" class:installed={!!fuelScoop}>
                    <span class="feature-icon">{fuelScoop ? '✅' : '⚪'}</span>
                    <div>
                      <strong>{fuelScoop ? fuelScoop.itemLabel : t('ship-no-fuel-scoop')}</strong>
                      {#if fuelScoop}<span class="sub mono">[{fuelScoop.sizeClass}]</span>{/if}
                    </div>
                  </div>
                  <div class="feature-item" class:installed={!!guardianBooster}>
                    <span class="feature-icon">{guardianBooster ? '✅' : '⚪'}</span>
                    <div>
                      <strong>{guardianBooster ? guardianBooster.itemLabel : t('ship-no-guardian-booster')}</strong>
                      {#if guardianBooster}<span class="sub mono cyan">+{guardianBooster.sizeClass.startsWith('5') ? '10.5' : 'AL'}</span>{/if}
                    </div>
                  </div>
                  <div class="feature-item" class:installed={!!dss}>
                    <span class="feature-icon">{dss ? '✅' : '⚪'}</span>
                    <div>
                      <strong>{dss ? dss.itemLabel : t('ship-no-dss')}</strong>
                      {#if dss?.engineering}<span class="sub mono gold">[{dss.engineering.blueprint}]</span>{/if}
                    </div>
                  </div>
                  <div class="feature-item" class:installed={!!srvBay}>
                    <span class="feature-icon">{srvBay ? '✅' : '⚪'}</span>
                    <div>
                      <strong>{srvBay ? srvBay.itemLabel : t('ship-no-srv-bay')}</strong>
                    </div>
                  </div>
                  <div class="feature-item" class:installed={!!afmu}>
                    <span class="feature-icon">{afmu ? '✅' : '⚪'}</span>
                    <div>
                      <strong>{afmu ? afmu.itemLabel : t('ship-no-afmu')}</strong>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          {:else if activeTab === 'combat'}
            <div class="stat-columns">
              <div class="stat-col">
                <div class="metric-card">
                  <span class="metric-label">{t('combat-shields')}</span>
                  <strong class="metric-val mono" class:cyan={combatShip?.shields_up} class:red={combatShip?.shields_up === false}>
                    {combatShip?.shields_up ? t('combat-shields-up') : t('combat-shields-down')}
                  </strong>
                </div>
                <div class="metric-card">
                  <span class="metric-label">{t('combat-heat')}</span>
                  <span class="metric-val mono" class:gold={combatShip?.heat === 'overheating'}>
                    {combatShip?.heat ?? t('combat-heat-normal')}
                  </span>
                </div>
                {#if combatShip?.pips}
                  <div class="metric-card">
                    <span class="metric-label">SYS / ENG / WPN</span>
                    <span class="metric-val mono cyan">
                      {combatShip.pips[0]} / {combatShip.pips[1]} / {combatShip.pips[2]}
                    </span>
                  </div>
                {/if}
              </div>

              <div class="stat-col combat-overview">
                <h4 class="sub-heading">{t('ship-combat-hardpoints')} ({hardpoints.length})</h4>
                <div class="compact-grid">
                  {#each hardpoints as hpt (hpt.slot)}
                    <div class="hpt-row">
                      <span class="hpt-slot sub mono">{hpt.slotLabel}</span>
                      <strong class="hpt-name">{hpt.itemLabel}</strong>
                      {#if hpt.ammo}<span class="hpt-ammo mono gold">[{hpt.ammo}]</span>{/if}
                    </div>
                  {/each}
                  {#if hardpoints.length === 0}
                    <p class="sub italic">{t('ship-no-hardpoints')}</p>
                  {/if}
                </div>

                <h4 class="sub-heading mt-2">{t('ship-combat-defenses')} ({utilities.length})</h4>
                <div class="defenses-badges">
                  <span class="hud-pill" class:hud-pill--active={shieldBoosters.length > 0}>
                    🛡️ {shieldBoosters.length} {t('ship-shield-boosters')}
                  </span>
                  <span class="hud-pill" class:hud-pill--active={chaff.length > 0}>
                    💨 {chaff.length} {t('ship-chaff')}
                  </span>
                  <span class="hud-pill" class:hud-pill--active={pointDefence.length > 0}>
                    🎯 {pointDefence.length} {t('ship-point-defence')}
                  </span>
                  <span class="hud-pill" class:hud-pill--active={heatSinks.length > 0}>
                    ❄️ {heatSinks.length} {t('ship-heat-sinks')}
                  </span>
                </div>
              </div>
            </div>
          {:else}
            <!-- Pestaña de Carga -->
            <div class="stat-columns">
              <div class="stat-col">
                <div class="metric-card">
                  <span class="metric-label">{t('field-cargo')} Total</span>
                  <strong class="metric-val mono gold">{ship.cargo_count ?? 0} / {ship.cargo_capacity ?? 0} t</strong>
                </div>
                <div class="metric-card">
                  <span class="metric-label">{t('ship-cargo-racks-count')}</span>
                  <span class="metric-val mono">{cargoRacks.length}</span>
                </div>
                {#if refinery}
                  <div class="metric-card">
                    <span class="metric-label">{t('ship-refinery')}</span>
                    <span class="metric-val mono cyan">{refinery.itemLabel}</span>
                  </div>
                {/if}
              </div>

              <div class="stat-col cargo-overview">
                <h4 class="sub-heading">{t('ship-limpets-controllers')} ({limpets.length})</h4>
                <div class="compact-grid">
                  {#each limpets as lmp (lmp.slot)}
                    <div class="hpt-row">
                      <span class="sub mono">{lmp.slotLabel}</span>
                      <strong>{lmp.itemLabel}</strong>
                    </div>
                  {/each}
                  {#if limpets.length === 0}
                    <p class="sub italic">{t('ship-no-limpets')}</p>
                  {/if}
                </div>

                {#if ship.cargo && ship.cargo.length > 0}
                  <h4 class="sub-heading mt-2">{t('ship-cargo-inventory')} ({ship.cargo.length})</h4>
                  <div class="cargo-items-list">
                    {#each ship.cargo as cItem (cItem.Name)}
                      <div class="cargo-item-row">
                        <span>{cItem.Name_Localised || cItem.Name}</span>
                        <strong class="mono gold">{cItem.Count} t</strong>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <!-- 3. DESGLOSE EXHAUSTIVO MODULO POR MODULO -->
      <div class="hud-card modules-section">
        <div class="section-header">
          <button
            type="button"
            class="header-toggle"
            onclick={() => modulesCollapsed = !modulesCollapsed}
            aria-expanded={!modulesCollapsed}
          >
            <h3 class="section-title">
              📋 {t('ship-modules-breakdown')}
              <span class="count-chip mono">({formattedModules.length})</span>
            </h3>
            <span class="chevron" aria-hidden="true">{modulesCollapsed ? '▼' : '▲'}</span>
          </button>
        </div>

        {#if !modulesCollapsed}
          <!-- Barra de Filtros y Buscador -->
          <div class="filter-bar">
            <div class="category-buttons" role="group">
              <button
                type="button"
                class="filter-pill"
                class:active={categoryFilter === 'all'}
                onclick={() => categoryFilter = 'all'}
              >
                {t('ship-filter-all')} ({formattedModules.length})
              </button>
              <button
                type="button"
                class="filter-pill"
                class:active={categoryFilter === 'hardpoint'}
                onclick={() => categoryFilter = 'hardpoint'}
              >
                {t('ship-filter-hardpoints')} ({hardpoints.length})
              </button>
              <button
                type="button"
                class="filter-pill"
                class:active={categoryFilter === 'core'}
                onclick={() => categoryFilter = 'core'}
              >
                {t('ship-filter-core')} ({formattedModules.filter((m) => m.category === 'core').length})
              </button>
              <button
                type="button"
                class="filter-pill"
                class:active={categoryFilter === 'optional'}
                onclick={() => categoryFilter = 'optional'}
              >
                {t('ship-filter-optional')} ({formattedModules.filter((m) => m.category === 'optional').length})
              </button>
              <button
                type="button"
                class="filter-pill"
                class:active={categoryFilter === 'utility'}
                onclick={() => categoryFilter = 'utility'}
              >
                {t('ship-filter-utility')} ({utilities.length})
              </button>
            </div>

            <input
              type="search"
              class="search-input"
              placeholder={t('ship-search-modules-placeholder')}
              bind:value={searchQuery}
            />
          </div>

          <!-- Lista de Tarjetas de Módulos -->
          <div class="modules-grid">
            {#each filteredModules as mod (mod.slot)}
              <div class="module-card" class:damaged={mod.healthPct < 100}>
                <div
                  class="module-header"
                  onclick={() => mod.engineering?.modifiers && toggleModuleDetails(mod.slot)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleModuleDetails(mod.slot); } }}
                >
                  <div class="mod-main">
                    <span class="slot-badge sub mono">{mod.slotLabel}</span>
                    <strong class="item-name">
                      {mod.itemLabel}
                      {#if mod.sizeClass}<span class="size-class mono cyan">[{mod.sizeClass}]</span>{/if}
                    </strong>
                  </div>

                  <div class="mod-status-row">
                    <!-- Salud con barra visual -->
                    <div class="mod-health" title="Salud del módulo">
                      <div class="health-track">
                        <div
                          class="health-fill"
                          class:health-fill--danger={mod.healthPct < 50}
                          class:health-fill--warning={mod.healthPct >= 50 && mod.healthPct < 100}
                          style="width: {mod.healthPct}%"
                        ></div>
                      </div>
                      <span class="health-val mono" class:warning={mod.healthPct < 100}>{mod.healthPct}%</span>
                    </div>

                    <!-- Píldoras de Energía / Prioridad -->
                    <span class="pwr-pill mono" class:pwr-on={mod.on} class:pwr-off={!mod.on}>
                      {mod.on ? 'ON' : 'OFF'}
                    </span>
                    <span class="pwr-prio mono" title="Prioridad de energía">P{mod.priority}</span>

                    <!-- Indicador de Ingeniería -->
                    {#if mod.engineering}
                      <span class="eng-pill gold" title="{mod.engineering.blueprint} G{mod.engineering.level}">
                        <GradeBadge grade={mod.engineering.level} size={14} />
                        <span class="eng-title">G{mod.engineering.level}</span>
                        {#if mod.engineering.experimental}
                          <span class="exp-badge">★</span>
                        {/if}
                        {#if mod.engineering.modifiers && mod.engineering.modifiers.length > 0}
                          <span class="chevron-sm">{expandedModuleSlot === mod.slot ? '▲' : '▼'}</span>
                        {/if}
                      </span>
                    {/if}
                  </div>
                </div>

                <!-- Desglose de Modificadores de Ingeniería al hacer clic -->
                {#if expandedModuleSlot === mod.slot && mod.engineering}
                  <div class="eng-details">
                    <div class="eng-meta-row">
                      <span class="cyan"><strong>{mod.engineering.blueprint}</strong> (Grado {mod.engineering.level})</span>
                      {#if mod.engineering.experimental}
                        <span class="gold">Efecto: <em>{mod.engineering.experimental}</em></span>
                      {/if}
                      {#if mod.engineering.engineer}
                        <span class="sub">Por: {mod.engineering.engineer}</span>
                      {/if}
                    </div>

                    {#if mod.engineering.modifiers && mod.engineering.modifiers.length > 0}
                      <div class="modifiers-table">
                        {#each mod.engineering.modifiers as m (m.label)}
                          <div class="modifier-row">
                            <span class="sub">{m.label}</span>
                            <span class="mono" class:cyan={!m.lessIsGood && (m.value ?? 0) >= (m.original ?? 0)}>
                              {m.value != null ? m.value.toFixed(2) : '—'}
                              {#if m.original != null}
                                <span class="sub-orig">({m.original.toFixed(2)})</span>
                              {/if}
                            </span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}

            {#if filteredModules.length === 0}
              <p class="empty-list sub italic">{t('ship-no-modules-match')}</p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</Panel>

<style>
  .ship-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .hud-card {
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-radius: 6px;
    padding: 1rem;
    flex-shrink: 0;
  }

  /* Cabecera y medidores */
  .cockpit-header {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  @media (min-width: 900px) {
    .cockpit-header {
      flex-direction: row;
      justify-content: space-between;
      align-items: center;
    }
  }

  .identity-col {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .ship-titles {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .ship-type {
    margin: 0;
    font-size: 1.4rem;
    color: var(--hud-primary);
  }

  .ship-name {
    font-size: 0.95rem;
    color: var(--hud-text);
  }

  .ship-ident {
    color: var(--hud-muted);
    margin-left: 0.35rem;
  }

  .badge-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .gauges-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 0.85rem;
    flex: 1;
    max-width: 720px;
  }

  .gauge-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .gauge-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
  }

  .gauge-label {
    color: var(--hud-muted);
  }

  .meter-track {
    height: 7px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--hud-border);
    border-radius: 2px;
    overflow: hidden;
  }

  .reserve-track {
    height: 3px;
    margin-top: 1px;
  }

  .meter-fill {
    height: 100%;
    background: var(--hud-primary);
    transition: width 150ms ease;
  }

  .meter-fill--danger {
    background: var(--hud-danger);
  }

  .meter-fill--warning {
    background: #ffb833;
  }

  .meter-fill--cyan {
    background: var(--hud-secondary);
  }

  .meter-fill--gold {
    background: #ffd043;
  }

  .reserve-fill {
    background: var(--hud-secondary);
  }

  /* Pestañas de Estadísticas */
  .stats-card {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .tab-buttons {
    display: flex;
    gap: 0.5rem;
    border-bottom: 1px solid var(--hud-border);
    padding-bottom: 0.5rem;
  }

  .tab-btn {
    background: transparent;
    border: 1px solid transparent;
    color: var(--hud-muted);
    font-size: 0.85rem;
    padding: 0.35rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    transition: all 120ms ease;
  }

  .tab-btn.active {
    background: rgba(255, 128, 0, 0.15);
    border-color: var(--hud-primary);
    color: var(--hud-primary);
    font-weight: bold;
  }

  .stat-columns {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1.25rem;
  }

  @media (min-width: 800px) {
    .stat-columns {
      grid-template-columns: 240px 1fr;
    }
  }

  .stat-col {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .metric-card {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .metric-label {
    font-size: 0.8rem;
    color: var(--hud-muted);
  }

  .metric-val {
    font-size: 1rem;
    color: var(--hud-text);
  }

  .sub-heading {
    margin: 0 0 0.4rem 0;
    font-size: 0.85rem;
    color: var(--hud-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .feature-pills {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .feature-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 0.35rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    flex-shrink: 0;
  }

  .feature-item.installed {
    border-color: rgba(0, 210, 255, 0.3);
  }

  .compact-grid {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .hpt-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    font-size: 0.8rem;
    background: rgba(0, 0, 0, 0.2);
    padding: 0.3rem 0.5rem;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .defenses-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .cargo-items-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 200px;
    overflow-y: auto;
    overflow-x: hidden;
    padding-right: 0.35rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.2);
  }

  .cargo-items-list::-webkit-scrollbar {
    width: 6px;
  }

  .cargo-items-list::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
  }

  .cargo-items-list::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.45);
    border-radius: 3px;
  }

  .cargo-items-list::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }

  .cargo-item-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
    background: rgba(0, 0, 0, 0.2);
    padding: 0.3rem 0.6rem;
    border-radius: 3px;
    flex-shrink: 0;
  }

  /* Sección Módulo por Módulo */
  .modules-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-header {
    border-bottom: 1px solid var(--hud-border);
    padding-bottom: 0.5rem;
  }

  .header-toggle {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0;
  }

  .section-title {
    margin: 0;
    font-size: 1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--hud-primary);
  }

  .count-chip {
    font-size: 0.8rem;
    color: var(--hud-muted);
  }

  .filter-bar {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  @media (min-width: 768px) {
    .filter-bar {
      flex-direction: row;
      justify-content: space-between;
      align-items: center;
    }
  }

  .category-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .filter-pill {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--hud-border);
    color: var(--hud-muted);
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
    border-radius: 12px;
    cursor: pointer;
    transition: all 120ms ease;
  }

  .filter-pill.active {
    background: var(--hud-primary);
    color: #000;
    border-color: var(--hud-primary);
    font-weight: bold;
  }

  .search-input {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--hud-border);
    color: var(--hud-text);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    min-width: 200px;
  }

  .modules-grid {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    max-height: 520px;
    overflow-y: auto;
    overflow-x: hidden;
    padding-right: 0.35rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.2);
  }

  .modules-grid::-webkit-scrollbar {
    width: 6px;
  }

  .modules-grid::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
  }

  .modules-grid::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.45);
    border-radius: 3px;
  }

  .modules-grid::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }

  .module-card {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
    overflow: hidden;
    flex-shrink: 0;
    min-height: 38px;
  }

  .module-card.damaged {
    border-left: 3px solid #ffb833;
  }

  .module-header {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.5rem 0.75rem;
    cursor: pointer;
  }

  @media (min-width: 768px) {
    .module-header {
      flex-direction: row;
      justify-content: space-between;
      align-items: center;
    }
  }

  .mod-main {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .slot-badge {
    color: var(--hud-muted);
    font-size: 0.75rem;
    min-width: 150px;
  }

  .item-name {
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .mod-status-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .mod-health {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
  }

  .health-track {
    width: 50px;
    height: 5px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--hud-border);
    border-radius: 2px;
    overflow: hidden;
  }

  .health-fill {
    height: 100%;
    background: var(--hud-primary);
  }

  .health-fill--danger { background: var(--hud-danger); }
  .health-fill--warning { background: #ffb833; }

  .pwr-pill {
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    border: 1px solid;
  }

  .pwr-on {
    background: rgba(0, 255, 128, 0.15);
    border-color: #00ff80;
    color: #00ff80;
  }

  .pwr-off {
    background: rgba(120, 120, 120, 0.2);
    border-color: #666;
    color: #888;
  }

  .pwr-prio {
    font-size: 0.7rem;
    color: var(--hud-muted);
  }

  .eng-pill {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.75rem;
    background: rgba(255, 208, 67, 0.1);
    border: 1px solid rgba(255, 208, 67, 0.3);
    padding: 0.15rem 0.45rem;
    border-radius: 3px;
  }

  .eng-details {
    padding: 0.6rem 0.75rem;
    background: rgba(0, 0, 0, 0.4);
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .eng-meta-row {
    display: flex;
    gap: 0.85rem;
    flex-wrap: wrap;
  }

  .modifiers-table {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.35rem 0.75rem;
    margin-top: 0.25rem;
  }

  .modifier-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    padding: 0.15rem 0.4rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 2px;
  }

  .sub-orig {
    color: var(--hud-muted);
    font-size: 0.7rem;
    margin-left: 0.25rem;
  }

  .mono { font-family: var(--font-mono); }
  .cyan { color: var(--hud-secondary); }
  .gold { color: #ffd043; }
  .red { color: var(--hud-danger); }
  .sub { color: var(--hud-muted); font-size: 0.8rem; }
  .italic { font-style: italic; }
  .mt-2 { margin-top: 0.6rem; }
  .hud-pill--active { border-color: var(--hud-secondary); color: var(--hud-secondary); }
  .empty { color: var(--hud-muted); margin: 0; padding: 1rem 0; }
  .empty-list { padding: 1rem; text-align: center; }
</style>
