<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal } from '../../lib/format';
  import { topic, action } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import {
    MINING_PRESETS,
    type MiningTab,
    type MiningPreset,
    type MiningTargetsTopic,
    type MiningLoopTopic,
    type MiningCargoSellTopic,
    type MiningRingTarget,
  } from './miningTargetsTypes';

  const targetsTopic = $derived(topic<MiningTargetsTopic>('mining.targets'));
  const loopTopic = $derived(topic<MiningLoopTopic>('mining.loop'));
  const cargoSellTopic = $derived(topic<MiningCargoSellTopic>('mining.cargo_sell'));

  // Ubicación actual desde navegación o dashboard
  const navSystem = $derived(topic<{ system?: string }>('navigation.jump')?.system);
  const locationSystem = $derived(topic<{ system?: string }>('dashboard.location')?.system);
  const currentSystem = $derived(navSystem ?? locationSystem ?? 'Sol');

  // Estado del formulario
  let selectedTab = $state<MiningTab>('laser');
  let selectedPresetId = $state('plat-laser');
  let originText = $state('');
  let originTouched = $state(false);
  let minHotspots = $state(1);
  let requireResCombo = $state(true);
  let copiedText = $state<string | null>(null);

  // Sincronizar origen inicial con la ubicación actual
  $effect(() => {
    if (!originTouched && currentSystem && !originText) {
      originText = currentSystem;
    }
  });

  // Filtrar presets por pestaña
  const activePresets = $derived(MINING_PRESETS.filter((p) => p.tab === selectedTab));
  const activePreset = $derived(
    MINING_PRESETS.find((p) => p.id === selectedPresetId) ?? activePresets[0],
  );

  function selectTab(tab: MiningTab) {
    selectedTab = tab;
    const firstPreset = MINING_PRESETS.find((p) => p.tab === tab);
    if (firstPreset) {
      selectPreset(firstPreset);
    }
  }

  function selectPreset(preset: MiningPreset) {
    selectedPresetId = preset.id;
    requireResCombo = Boolean(preset.defaultResCombo);
    // Ejecutar búsqueda inmediata
    search(preset.commodity);
  }

  function search(commodityName?: string) {
    const targetCommodity = commodityName ?? activePreset?.commodity ?? 'Platinum';
    void action('mining.search_targets', {
      commodity: targetCommodity,
      min_hotspots: minHotspots,
      require_res_combo: requireResCombo,
      reference_system: originText.trim() || currentSystem,
    });
  }

  function selectRing(ring: MiningRingTarget) {
    void action('mining.select_target', {
      system_name: ring.system_name,
      body_name: ring.body_name,
      ring_name: ring.ring_name,
    });
  }

  function searchMultiSell() {
    void action('mining.search_multi_sell', {});
  }

  function clearSearch() {
    void action('mining.clear_targets', {});
  }

  function copyToClipboard(text: string) {
    if (navigator?.clipboard) {
      navigator.clipboard.writeText(text).catch(() => {});
      copiedText = text;
      setTimeout(() => {
        if (copiedText === text) copiedText = null;
      }, 2000);
    }
  }

  function planRoute(sys: string) {
    void action('navigation.plan', {
      to: sys,
      mode: 'galaxy',
      use_ship: true,
    }).catch(() => {});
    copyToClipboard(sys);
  }

  function ringTypeLabel(rt: string): string {
    const lower = rt.toLowerCase();
    if (lower.includes('metallic')) return t('mining-ring-metallic');
    if (lower.includes('metal-rich') || lower.includes('metal_rich')) return t('mining-ring-metal-rich');
    if (lower.includes('icy')) return t('mining-ring-icy');
    if (lower.includes('rocky')) return t('mining-ring-rocky');
    return rt;
  }

  function resBadgeClass(res?: string | null): string {
    if (!res) return '';
    const l = res.toLowerCase();
    if (l.includes('hazard')) return 'badge-danger';
    if (l.includes('high')) return 'badge-warning';
    return 'badge-info';
  }

  function resLabel(res?: string | null): string {
    if (!res) return '';
    const l = res.toLowerCase();
    if (l.includes('hazard')) return t('mining-res-hazard');
    if (l.includes('high')) return t('mining-res-high');
    if (l.includes('low')) return t('mining-res-low');
    return t('mining-res-normal');
  }

  const loopPlan = $derived(loopTopic?.plan ?? null);
  const ringTargets = $derived(targetsTopic?.targets ?? []);
  const selectedRing = $derived(
    loopPlan ? ringTargets.find((r) => r.system_name === loopPlan.ring_target.system_name && r.ring_name === loopPlan.ring_target.ring_name) : ringTargets[0],
  );

  // Altura dinámica adaptativa de la tabla sin scroll fantasma (AGENTS.md §11.2)
  const listHeight = $derived(Math.max(120, Math.min(380, ringTargets.length * 44 + 40)));
</script>

<Panel titleKey="panel-mining-targets" span={2} collapsible={true} defaultExpanded={true}>
  <!-- ZONA SUPERIOR PRIORITARIA (~33 % INFO BÁSICA Y BUCLE TRIANGULAR DE CABINA) -->
  <div class="mining-cockpit-header">
    <!-- PESTAÑAS DE PERFIL -->
    <div class="profile-tabs" role="tablist">
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === 'laser'}
        onclick={() => selectTab('laser')}
        role="tab"
        aria-selected={selectedTab === 'laser'}
      >
        {t('mining-tab-laser')}
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === 'core'}
        onclick={() => selectTab('core')}
        role="tab"
        aria-selected={selectedTab === 'core'}
      >
        {t('mining-tab-core')}
      </button>
      <button
        type="button"
        class="tab-btn"
        class:active={selectedTab === 'missions'}
        onclick={() => selectTab('missions')}
        role="tab"
        aria-selected={selectedTab === 'missions'}
      >
        {t('mining-tab-missions')}
      </button>
    </div>

    <!-- PRESETS RÁPIDOS EN PÍLDORAS -->
    <div class="presets-row">
      {#each activePresets as preset (preset.id)}
        <button
          type="button"
          class="preset-chip"
          class:active={selectedPresetId === preset.id}
          onclick={() => selectPreset(preset)}
          title="{preset.methodLabel} · {ringTypeLabel(preset.ringType)}"
        >
          <span class="preset-name">{preset.commodity}</span>
          <span class="preset-ring-tag">{ringTypeLabel(preset.ringType)}</span>
        </button>
      {/each}
    </div>

    <!-- BARRA DE CONTROLES Y FILTROS -->
    <div class="controls-bar">
      <div class="input-group">
        <label for="mining-origin-input">{t('mining-ref-system')}</label>
        <input
          id="mining-origin-input"
          type="text"
          class="hud-input"
          bind:value={originText}
          oninput={() => (originTouched = true)}
          placeholder={currentSystem}
        />
      </div>

      <div class="filter-group">
        <label for="mining-hotspots-select">{t('mining-min-hotspots', { count: minHotspots })}</label>
        <select id="mining-hotspots-select" class="hud-select" bind:value={minHotspots} onchange={() => search()}>
          <option value={1}>1+</option>
          <option value={2}>2+ (Solapado)</option>
          <option value={3}>3+ (Triple)</option>
        </select>
      </div>

      {#if selectedTab === 'laser'}
        <label class="switch-control">
          <input
            type="checkbox"
            class="switch-btn"
            bind:checked={requireResCombo}
            onchange={() => search()}
          />
          <span class="switch-label">{t('mining-filter-res-combo')}</span>
        </label>
      {/if}

      <div class="btn-group-row">
        <button type="button" class="btn-primary" onclick={() => search()} title={t('mining-search-btn')}>
          🔍 {t('mining-search-btn')}
        </button>
        <button type="button" class="btn-secondary" onclick={clearSearch} title={t('mining-clear-btn')}>
          {t('mining-clear-btn')}
        </button>
      </div>
    </div>

    <!-- TARJETA HUD DEL BUCLE TRIANGULAR MÁS CORTO -->
    {#if loopPlan}
      <div class="loop-hud-card" data-testid="loop-plan-card">
        <div class="loop-header">
          <span class="loop-title">🔄 {t('mining-shortest-loop')}</span>
          <span class="badge badge-accent">{t('mining-total-dist', { dist: loopPlan.total_distance_ly })}</span>
        </div>

        <div class="loop-pipeline">
          <!-- ETAPA 1: ORIGEN -->
          <div class="loop-step">
            <span class="step-label">1. {t('mining-origin', { system: loopPlan.origin_system })}</span>
            <div class="step-content">
              <span class="step-sys">{loopPlan.origin_system}</span>
            </div>
          </div>

          <div class="pipeline-arrow">➔</div>

          <!-- ETAPA 2: ANILLO OBJETIVO -->
          <div class="loop-step highlight-ring">
            <span class="step-label">2. {t('mining-ring-target', { body: loopPlan.ring_target.body_name, ring: loopPlan.ring_target.ring_name })}</span>
            <div class="step-content">
              <span class="step-sys">{loopPlan.ring_target.system_name}</span>
              <span class="step-badge badge-dist">{loopPlan.ring_target.distance_ly} al</span>
              {#if loopPlan.ring_target.res_intensity}
                <span class="badge {resBadgeClass(loopPlan.ring_target.res_intensity)}">
                  {resLabel(loopPlan.ring_target.res_intensity)}
                </span>
              {/if}
              <button
                type="button"
                class="btn-icon"
                onclick={() => copyToClipboard(loopPlan.ring_target.system_name)}
                title={t('mining-copy-system')}
              >
                {copiedText === loopPlan.ring_target.system_name ? '✓' : '📋'}
              </button>
              <button
                type="button"
                class="btn-icon"
                onclick={() => planRoute(loopPlan.ring_target.system_name)}
                title="Trazar ruta"
              >
                🚀
              </button>
            </div>
          </div>

          <div class="pipeline-arrow">➔</div>

          <!-- ETAPA 3: VENTA ÓPTIMA -->
          <div class="loop-step highlight-sell">
            <span class="step-label">3. {t('mining-sell-target', { station: loopPlan.sell_station.station_name, system: loopPlan.sell_station.system_name })}</span>
            <div class="step-content">
              <span class="step-station">{loopPlan.sell_station.station_name}</span>
              <span class="step-price">{credits(loopPlan.sell_station.price_cr)}/t</span>
              <span class="step-badge badge-dist">+{loopPlan.sell_station.distance_from_ring_ly} al</span>

              <!-- SEMÁFORO DE DEMANDA -->
              {#if loopPlan.sell_station.demand_tax_risk}
                <span
                  class="badge badge-warning"
                  title={t('mining-demand-tax-warning', { demand: loopPlan.sell_station.demand })}
                >
                  ⚠️ {t('mining-demand-tax-warning', { demand: loopPlan.sell_station.demand })}
                </span>
              {:else}
                <span class="badge badge-success" title={t('mining-demand-safe')}>
                  ✓ {t('mining-demand-safe')} ({loopPlan.sell_station.demand} t)
                </span>
              {/if}

              <button
                type="button"
                class="btn-icon"
                onclick={() => copyToClipboard(loopPlan.sell_station.system_name)}
                title={t('mining-copy-system')}
              >
                {copiedText === loopPlan.sell_station.system_name ? '✓' : '📋'}
              </button>
            </div>
          </div>
        </div>
      </div>
    {:else if targetsTopic?.status === 'loading'}
      <div class="status-banner loading">
        <span class="spinner">⏳</span> {t('mining-sell-loading')}
      </div>
    {:else if targetsTopic?.status === 'empty'}
      <div class="status-banner empty">
        {t('mining-no-targets')}
      </div>
    {/if}

    <!-- ACCESO RÁPIDO A VENTA MÚLTIPLE DE BODEGA -->
    {#if cargoSellTopic && cargoSellTopic.commodities_in_cargo.length > 0}
      <div class="cargo-sell-strip">
        <span class="cargo-strip-title">📦 Bodega de minería:</span>
        {#each cargoSellTopic.commodities_in_cargo as item (item.name)}
          <span class="badge badge-neutral">{item.name}: {item.count} t</span>
        {/each}
        <button type="button" class="btn-secondary" onclick={searchMultiSell}>
          {t('mining-cargo-sell-btn')}
        </button>
      </div>
    {/if}
  </div>

  <!-- SECCIÓN COLAPSABLE CON INFORMACIÓN DETALLADA (TABLA DE YACIMIENTOS Y MERCADOS) -->
  {#snippet details()}
    <div class="mining-details-section">
      <div class="section-header">
        <h3>{t('nav-mining-targets')} ({ringTargets.length})</h3>
        <span class="catalogue-provenance">
          {targetsTopic?.status === 'ok' ? t('mining-online-updated') : t('mining-offline-catalog')}
        </span>
      </div>

      {#if ringTargets.length > 0}
        <div class="table-container" style:height="{listHeight}px">
          <table class="hud-table" data-testid="mining-targets-table">
            <thead>
              <tr>
                <th>{t('field-system')}</th>
                <th>{t('field-body')}</th>
                <th>Tipo Anillo</th>
                <th>Hotspots</th>
                <th>Rendimiento / RES</th>
                <th>Notas / Solapamiento</th>
                <th>Acción</th>
              </tr>
            </thead>
            <tbody>
              {#each ringTargets as target (target.system_name + target.body_name + target.ring_name)}
                <tr class:selected-row={selectedRing?.system_name === target.system_name && selectedRing?.ring_name === target.ring_name}>
                  <td class="cell-system">
                    <span class="sys-name">{target.system_name}</span>
                    <span class="sys-dist">{target.distance_ly} al</span>
                    <button
                      type="button"
                      class="btn-icon"
                      onclick={() => copyToClipboard(target.system_name)}
                      title={t('mining-copy-system')}
                    >
                      {copiedText === target.system_name ? '✓' : '📋'}
                    </button>
                  </td>
                  <td>
                    <span class="body-name">{target.body_name}</span>
                    <span class="ring-name">{target.ring_name}</span>
                  </td>
                  <td>
                    <span class="badge badge-subtle">{ringTypeLabel(target.ring_type)}</span>
                  </td>
                  <td>
                    <span class="badge badge-accent">{target.hotspot_count}x</span>
                  </td>
                  <td>
                    {#if target.res_intensity}
                      <span class="badge {resBadgeClass(target.res_intensity)}">
                        {resLabel(target.res_intensity)}
                      </span>
                    {:else}
                      <span class="muted">-</span>
                    {/if}
                  </td>
                  <td class="cell-notes">
                    {#if target.comment}
                      <span class="comment-text" title={target.comment}>{target.comment}</span>
                    {:else}
                      <span class="muted">-</span>
                    {/if}
                  </td>
                  <td>
                    {#if selectedRing?.system_name === target.system_name && selectedRing?.ring_name === target.ring_name}
                      <span class="badge badge-success">{t('mining-selected')}</span>
                    {:else}
                      <button
                        type="button"
                        class="btn-select"
                        onclick={() => selectRing(target)}
                      >
                        {t('mining-select-ring')}
                      </button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <!-- Barra de scroll horizontal pegajosa al fondo (AGENTS.md §11.3) -->
          <div class="sticky-hscroll-anchor"></div>
        </div>
      {:else}
        <p class="muted">{t('mining-no-targets')}</p>
      {/if}

      <!-- TABLA DE MERCADOS MULTI-VENTA EN BODEGA -->
      {#if cargoSellTopic && cargoSellTopic.best_multi_stations.length > 0}
        <div class="multi-sell-section">
          <h3>🏪 {t('mining-multi-sell')}</h3>
          <p class="section-desc">{t('mining-multi-sell-desc')}</p>

          <div class="table-container" style="max-height: 220px;">
            <table class="hud-table">
              <thead>
                <tr>
                  <th>{t('field-station')}</th>
                  <th>{t('field-system')}</th>
                  <th>{t('mining-col-pad')}</th>
                  <th>{t('mining-col-distance')}</th>
                  <th>Ingresos Totales</th>
                  <th>Acción</th>
                </tr>
              </thead>
              <tbody>
                {#each cargoSellTopic.best_multi_stations as st (st.station_name + st.system_name)}
                  <tr>
                    <td class="bold">{st.station_name}</td>
                    <td>{st.system_name}</td>
                    <td><span class="badge">{st.pad}</span></td>
                    <td>{st.distance_ly} al</td>
                    <td class="price-val">{credits(st.total_revenue_cr)}</td>
                    <td>
                      <button
                        type="button"
                        class="btn-icon"
                        onclick={() => copyToClipboard(st.system_name)}
                        title={t('mining-copy-system')}
                      >
                        {copiedText === st.system_name ? '✓' : '📋'}
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </div>
  {/snippet}
</Panel>

<style>
  .mining-cockpit-header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .profile-tabs {
    display: flex;
    gap: 0.5rem;
    border-bottom: 1px solid rgba(255, 160, 0, 0.2);
    padding-bottom: 0.25rem;
  }

  .tab-btn {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--hud-muted, #888);
    padding: 0.4rem 0.8rem;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .tab-btn:hover {
    color: var(--hud-text, #fff);
  }

  .tab-btn.active {
    color: var(--hud-orange, #ff9000);
    border-bottom-color: var(--hud-orange, #ff9000);
  }

  .presets-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .preset-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.6rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: var(--hud-text, #ddd);
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .preset-chip:hover {
    background: rgba(255, 144, 0, 0.1);
    border-color: rgba(255, 144, 0, 0.4);
    color: #fff;
  }

  .preset-chip.active {
    background: rgba(255, 144, 0, 0.2);
    border-color: var(--hud-orange, #ff9000);
    color: var(--hud-orange, #ff9000);
    font-weight: 600;
  }

  .preset-ring-tag {
    font-size: 0.75rem;
    opacity: 0.7;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.1rem 0.3rem;
    border-radius: 2px;
  }

  .controls-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 0.75rem;
    background: rgba(0, 0, 0, 0.25);
    padding: 0.6rem;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .input-group,
  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .input-group label,
  .filter-group label {
    font-size: 0.75rem;
    color: var(--hud-muted, #aaa);
    text-transform: uppercase;
  }

  .hud-input,
  .hud-select {
    background: rgba(20, 20, 20, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #fff;
    padding: 0.35rem 0.6rem;
    border-radius: 3px;
    font-size: 0.85rem;
  }

  .switch-control {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.35rem;
    cursor: pointer;
    flex-shrink: 0;
  }

  .switch-btn {
    cursor: pointer;
    accent-color: var(--hud-orange, #ff9000);
  }

  .switch-label {
    font-size: 0.8rem;
    color: var(--hud-text, #ccc);
  }

  .btn-group-row {
    display: flex;
    gap: 0.4rem;
    margin-left: auto;
  }

  .btn-primary,
  .btn-secondary {
    padding: 0.35rem 0.75rem;
    border-radius: 3px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .btn-primary {
    background: var(--hud-orange, #ff9000);
    color: #000;
  }

  .btn-primary:hover {
    background: #ffa726;
  }

  .btn-secondary {
    background: rgba(255, 255, 255, 0.1);
    color: #ddd;
    border-color: rgba(255, 255, 255, 0.2);
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.2);
    color: #fff;
  }

  /* TARJETA HUD DEL BUCLE TRIANGULAR */
  .loop-hud-card {
    background: linear-gradient(135deg, rgba(255, 144, 0, 0.08), rgba(0, 0, 0, 0.4));
    border: 1px solid rgba(255, 144, 0, 0.3);
    border-radius: 6px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .loop-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 144, 0, 0.15);
    padding-bottom: 0.35rem;
  }

  .loop-title {
    font-weight: 700;
    font-size: 0.9rem;
    color: var(--hud-orange, #ff9000);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .loop-pipeline {
    display: grid;
    grid-template-columns: 1fr auto 1.5fr auto 1.5fr;
    align-items: center;
    gap: 0.5rem;
  }

  .loop-step {
    background: rgba(0, 0, 0, 0.3);
    padding: 0.5rem;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .loop-step.highlight-ring {
    border-color: rgba(33, 150, 243, 0.4);
    background: rgba(33, 150, 243, 0.05);
  }

  .loop-step.highlight-sell {
    border-color: rgba(76, 175, 80, 0.4);
    background: rgba(76, 175, 80, 0.05);
  }

  .step-label {
    font-size: 0.7rem;
    color: var(--hud-muted, #aaa);
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .step-content {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .step-sys,
  .step-station {
    font-weight: 700;
    font-size: 0.9rem;
    color: #fff;
  }

  .step-price {
    font-weight: 700;
    color: #69f0ae;
    font-size: 0.85rem;
  }

  .step-badge {
    font-size: 0.75rem;
    padding: 0.1rem 0.3rem;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.1);
  }

  .pipeline-arrow {
    color: var(--hud-orange, #ff9000);
    font-size: 1.1rem;
    opacity: 0.8;
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-weight: 600;
  }

  .badge-danger {
    background: rgba(244, 67, 54, 0.2);
    border: 1px solid rgba(244, 67, 54, 0.5);
    color: #ff8a80;
  }

  .badge-warning {
    background: rgba(255, 152, 0, 0.2);
    border: 1px solid rgba(255, 152, 0, 0.5);
    color: #ffd180;
  }

  .badge-success {
    background: rgba(76, 175, 80, 0.2);
    border: 1px solid rgba(76, 175, 80, 0.5);
    color: #b9f6ca;
  }

  .badge-accent {
    background: rgba(255, 144, 0, 0.2);
    border: 1px solid var(--hud-orange, #ff9000);
    color: var(--hud-orange, #ff9000);
  }

  .badge-subtle {
    background: rgba(255, 255, 255, 0.08);
    color: #ccc;
  }

  .btn-icon {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 3px;
    color: #ddd;
    font-size: 0.75rem;
    padding: 0.15rem 0.35rem;
    cursor: pointer;
  }

  .btn-icon:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
  }

  .cargo-sell-strip {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    border-left: 3px solid #69f0ae;
    font-size: 0.85rem;
  }

  .status-banner {
    padding: 0.6rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .status-banner.loading {
    background: rgba(33, 150, 243, 0.1);
    color: #90caf9;
  }

  .status-banner.empty {
    background: rgba(255, 255, 255, 0.05);
    color: #bbb;
  }

  /* DETALLES Y TABLAS */
  .mining-details-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-header h3 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--hud-text, #fff);
  }

  .catalogue-provenance {
    font-size: 0.75rem;
    color: var(--hud-muted, #888);
  }

  .table-container {
    overflow-y: auto;
    overflow-x: auto;
    position: relative;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.25);
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 144, 0, 0.4) transparent;
  }

  .hud-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
    min-width: 680px;
  }

  .hud-table thead {
    position: sticky;
    top: 0;
    background: #111;
    z-index: 5;
    box-shadow: 0 1px 0 rgba(255, 144, 0, 0.3);
  }

  .hud-table th,
  .hud-table td {
    padding: 0.4rem 0.6rem;
    text-align: left;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .hud-table th {
    color: var(--hud-muted, #aaa);
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  .selected-row {
    background: rgba(255, 144, 0, 0.1);
  }

  .cell-system {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .sys-name {
    font-weight: 700;
    color: #fff;
  }

  .sys-dist {
    font-size: 0.75rem;
    color: var(--hud-muted, #888);
  }

  .body-name {
    font-weight: 600;
  }

  .ring-name {
    font-size: 0.75rem;
    color: var(--hud-muted, #aaa);
    margin-left: 0.25rem;
  }

  .cell-notes {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .comment-text {
    font-size: 0.75rem;
    color: var(--hud-text, #bbb);
  }

  .btn-select {
    background: rgba(255, 144, 0, 0.15);
    border: 1px solid var(--hud-orange, #ff9000);
    color: var(--hud-orange, #ff9000);
    font-size: 0.75rem;
    padding: 0.15rem 0.5rem;
    border-radius: 2px;
    cursor: pointer;
  }

  .btn-select:hover {
    background: var(--hud-orange, #ff9000);
    color: #000;
  }

  .sticky-hscroll-anchor {
    position: sticky;
    bottom: 0;
    height: 1px;
    z-index: 10;
  }

  .multi-sell-section {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .multi-sell-section h3 {
    margin: 0;
    font-size: 0.9rem;
    color: #69f0ae;
  }

  .section-desc {
    margin: 0;
    font-size: 0.75rem;
    color: var(--hud-muted, #888);
  }

  .price-val {
    color: #69f0ae;
    font-weight: 700;
  }

  .bold {
    font-weight: 700;
  }

  .muted {
    color: var(--hud-muted, #777);
  }
</style>
