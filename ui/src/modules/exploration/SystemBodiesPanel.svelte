<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, isScoopable, num1, pct, starClassLabel } from '../../lib/format';
  import { prefs } from '../../lib/prefs.svelte';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import VirtualList from '../../shell/VirtualList.svelte';
  import { getCardBox, listSpace } from '../../shell/cardBox';
  import { pushToast } from '../../shell/Toasts.svelte';
  import type { BodyRow, SystemTopic, UnsoldTopic } from './types';

  interface LocationTopic {
    system?: string;
    system_address?: number;
    star_pos?: [number, number, number];
    body?: string | null;
    body_id?: number | null;
  }

  const system = $derived(topic<SystemTopic>('exploration.system'));
  const unsold = $derived(topic<UnsoldTopic>('exploration.unsold'));
  const loc = $derived(topic<LocationTopic>('core.location'));
  const progress = $derived(system?.fss?.complete ? 1 : Math.max(0, Math.min(1, system?.fss?.progress ?? 0)));
  const columns = ['body', 'type', 'distance', 'scanned', 'mapped', 'signals', 'marks', 'advice'];
  const markKeys = ['terraformable', 'landable', 'scoopable', 'first-discovery', 'already-mapped', 'mapped-by-you'];
  function marks(body: BodyRow): string {
    return markKeys.filter((_, index) => ((body.marks ?? 0) & (1 << index)) !== 0)
      .map(key => t(`exploration-mark-${key}`)).join(' · ') || '—';
  }
  function advice(body: BodyRow): string {
    return (body.advice ?? ['not_worth']).map(key => t(`exploration-advice-${key.replaceAll('_', '-')}`)).join(' · ');
  }

  let tableScrollEl = $state<HTMLDivElement>();
  let stickyScrollEl = $state<HTMLDivElement>();
  let tableEl = $state<HTMLDivElement>();
  let tableScrollWidth = $state(1350);
  let hasHorizontalOverflow = $state(true);
  let syncing = false;
  let copied = $state(false);

  const rowCount = $derived(system?.rows?.length ?? 0);
  // Con alto fijo de tarjeta la lista ocupa el espacio medido que queda libre en el cuerpo del panel
  // (sin pasar de sus filas): ni scroll fantasma ni dos scrolls compitiendo.
  const cardBox = getCardBox();
  let listSpacePx = $state<number | null>(null);
  const baseListHeight = $derived(Math.max(30, Math.min(420, rowCount * 30)));
  const listHeight = $derived(cardBox?.height == null || listSpacePx === null ? baseListHeight
    : Math.max(baseListHeight, Math.min(rowCount * 30, listSpacePx)));

  $effect(() => {
    // rowCount como dependencia: se vuelve a medir cuando cambian las filas (y la lista ya existe).
    void rowCount;
    if (cardBox?.height == null || !tableEl) {
      listSpacePx = null;
      return;
    }
    const body = tableEl.closest('.hud-panel-body') as HTMLElement | null;
    const list = tableEl.querySelector('[data-testid="virtual-list"]') as HTMLElement | null;
    if (!body || !list) return;
    const measure = () => {
      const space = listSpace(list, body);
      if (space !== null) listSpacePx = space;
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    for (const el of [body, list, body.querySelector('.hud-panel-basic'), tableEl.closest('.table-container')]) {
      if (el) ro.observe(el);
    }
    return () => ro.disconnect();
  });

  const distSol = $derived.by(() => {
    if (!loc?.star_pos || loc.star_pos.length < 3) return null;
    const [x, y, z] = loc.star_pos;
    return Math.hypot(x, y, z).toFixed(1);
  });

  const distBase = $derived.by(() => {
    if (!loc?.star_pos || loc.star_pos.length < 3 || !prefs.homeSystemCoords || prefs.homeSystemCoords.length < 3) return null;
    const [x1, y1, z1] = loc.star_pos;
    const [x2, y2, z2] = prefs.homeSystemCoords;
    return Math.hypot(x1 - x2, y1 - y2, z1 - z2).toFixed(1);
  });

  const isCurrentSystemHome = $derived(
    Boolean(
      loc?.system &&
      prefs.homeSystem &&
      loc.system.trim().toLowerCase() === prefs.homeSystem.trim().toLowerCase()
    )
  );

  const terraformableCount = $derived(
    system?.terraformable_count ?? 
    (system?.rows?.filter(r => ((r.marks ?? 0) & 1) !== 0).length ?? 0)
  );
  const landableCount = $derived(
    system?.landable_count ?? 
    (system?.rows?.filter(r => ((r.marks ?? 0) & 2) !== 0).length ?? 0)
  );
  const bioCount = $derived(
    system?.bio_signals ?? 
    (system?.rows?.reduce((acc, r) => acc + (r.bio ?? 0), 0) ?? 0)
  );
  const geoCount = $derived(
    system?.geo_signals ?? 
    (system?.rows?.reduce((acc, r) => acc + (r.geo ?? 0), 0) ?? 0)
  );

  async function copySystem() {
    if (!system?.system) return;
    try {
      await navigator.clipboard.writeText(system.system);
      copied = true;
      pushToast(t('action-copied'), 'Info');
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // ignore
    }
  }

  function isCurrentBody(body: BodyRow): boolean {
    if (!loc) return false;
    if (loc.body_id != null && loc.body_id === body.body_id) return true;
    if (loc.body && (loc.body === body.short_name || loc.body.endsWith(` ${body.short_name}`))) return true;
    return false;
  }

  function onTableScroll() {
    if (syncing || !tableScrollEl || !stickyScrollEl) return;
    syncing = true;
    stickyScrollEl.scrollLeft = tableScrollEl.scrollLeft;
    syncing = false;
  }

  function onStickyScroll() {
    if (syncing || !tableScrollEl || !stickyScrollEl) return;
    syncing = true;
    tableScrollEl.scrollLeft = stickyScrollEl.scrollLeft;
    syncing = false;
  }

  $effect(() => {
    if (tableEl && tableScrollEl) {
      tableScrollWidth = Math.max(1350, tableEl.scrollWidth);
      hasHorizontalOverflow = tableScrollEl.clientWidth === 0 || tableEl.scrollWidth > tableScrollEl.clientWidth + 1;
    }
  });
</script>

<Panel titleKey="panel-system-bodies" span="full" size="large" defaultExpanded={false}>
  {#if system}
    <div class="summary">
      <div class="system-meta-col">
        <div class="system-header">
          <strong class="system" data-testid="system-name">{system.system}</strong>
          <button
            type="button"
            class="copy-btn"
            onclick={copySystem}
            title={t(copied ? 'action-copied' : 'action-copy-system')}
            aria-label={t('action-copy-system')}
          >
            {#if copied}
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
            {:else}
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            {/if}
          </button>
          {#if system.discovered != null}
            <span class="badge" class:badge-green={system.discovered} class:badge-orange={!system.discovered}>
              {t(system.discovered ? 'exploration-discovered' : 'exploration-undiscovered')}
            </span>
          {/if}
        </div>
        <div class="system-sub">
          {#if system.system_address}
            <span class="sub-pill">{t('exploration-system-address')}: <span class="mono">#{system.system_address}</span></span>
          {/if}
          {#if isCurrentSystemHome}
            <span class="sub-pill home-pill">🏠 {t('field-home-base')}</span>
          {:else if distBase != null && prefs.homeSystem}
            <span class="sub-pill home-dist-pill">{distBase} AL {t('field-distance-base', { system: prefs.homeSystem })}</span>
          {/if}
          {#if distSol && (!prefs.homeSystem || prefs.homeSystem.toLowerCase() !== 'sol')}
            <span class="sub-pill">{t('exploration-distance-sol')}: <span class="mono">{distSol} AL</span></span>
          {/if}
        </div>
      </div>

      {#if system.star_classes && system.star_classes.length > 0}
        <div class="stars" data-testid="system-stars">
          <span class="stars-label">{t('exploration-stars')}:</span>
          <div class="star-list">
            {#each system.star_classes as star}
              <span
                class="star-badge"
                class:scoopable={isScoopable(star)}
                data-testid="system-star-badge"
                title="{starClassLabel(star)} · {t(isScoopable(star) ? 'strip-scoopable' : 'strip-not-scoopable')}"
              >
                {starClassLabel(star)}
              </span>
            {/each}
          </div>
        </div>
      {/if}

      <div class="telemetry-chips">
        {#if terraformableCount > 0}
          <span class="telemetry-chip terraformable" title="{terraformableCount} mundos terraformables">
            🌍 {t('exploration-terraformable-count', { count: terraformableCount })}
          </span>
        {/if}
        {#if landableCount > 0}
          <span class="telemetry-chip landable" title="{landableCount} cuerpos aterrizables">
            🛬 {t('exploration-landable-count', { count: landableCount })}
          </span>
        {/if}
        {#if bioCount > 0}
          <span class="telemetry-chip bio" title="{bioCount} señales biológicas">
            🌿 {t('exploration-bio-total', { count: bioCount })}
          </span>
        {/if}
        {#if geoCount > 0}
          <span class="telemetry-chip geo" title="{geoCount} señales geológicas">
            🌋 {t('exploration-geo-total', { count: geoCount })}
          </span>
        {/if}
        {#if (system.non_body_count ?? 0) > 0}
          <span class="telemetry-chip signals" title="{(system.non_body_count ?? 0)} señales espaciales">
            📡 {t('exploration-non-body-signals', { count: system.non_body_count ?? 0 })}
          </span>
        {/if}
      </div>

      <div class="fss-card">
        <div class="fss-label-row">
          <label for="exploration-fss" class="fss-title">{t('exploration-fss-progress')} {system?.fss?.complete ? pct(1) : pct(system?.fss?.progress ?? 0)}</label>
        </div>
        <progress id="exploration-fss" value={progress} max="1"></progress>
        <span class="fss-count">{t('exploration-scanned-count', { scanned: system?.fss?.scanned ?? 0, total: system?.fss?.body_count ?? '—' })}</span>
      </div>

      <div class="metric-card value-card">
        <span class="metric-label">{t('exploration-value-now')}</span>
        <strong class="metric-val gold">{credits(system.value_now)}</strong>
        <span class="metric-sub">{t('exploration-scanned-bodies', { count: system.fss?.scanned ?? 0 })}</span>
      </div>
      <div class="metric-card value-card">
        <span class="metric-label">{t('exploration-value-if-mapped')}</span>
        {#if (system.map_targets ?? 0) > 0 || (system.value_if_mapped ?? 0) > (system.value_now ?? 0)}
          <strong class="metric-val green">{credits(system.value_if_mapped)}</strong>
          <span class="metric-sub green">+{credits((system.value_if_mapped ?? 0) - (system.value_now ?? 0))} · {t('exploration-map-targets-count', { count: system.map_targets ?? 1 })}</span>
        {:else}
          <strong class="metric-val text-dim">{credits(system.value_now)}</strong>
          <span class="metric-sub text-dim">{t('exploration-no-map-targets')}</span>
        {/if}
      </div>
    </div>
  {:else}
    <p class="empty">{t('empty-no-data')}</p>
  {/if}

  {#snippet details()}
    {#if system}
      {#if (system.truncated ?? 0) > 0}
        <p data-testid="exploration-truncated">{t('exploration-truncated', { count: system.truncated ?? 0 })}</p>
      {/if}
      <div class="table-container">
        <div class="table-scroll" bind:this={tableScrollEl} onscroll={onTableScroll} class:no-overflow={!hasHorizontalOverflow}>
          <div role="table" aria-label={t('panel-system-bodies')} aria-rowcount={(system.rows?.length ?? 0) + 1} class="body-table" bind:this={tableEl}>
            <div class="body-grid headings" role="row">
              {#each columns as column}<span role="columnheader">{t(`exploration-col-${column}`)}</span>{/each}
            </div>
            <VirtualList items={system.rows ?? []} rowHeight={30} height={listHeight} label={t('panel-system-bodies')}>
              {#snippet row(body, index)}
                {@const current = isCurrentBody(body)}
                <div role="row" aria-rowindex={index + 2} class="body-grid body-row" class:map-advice={body.advice?.includes('map')} class:current-body-row={current}
                  data-testid="body-row" data-body-id={body.body_id}>
                  <span role="cell" class="cell-body-name" title={body.short_name}>
                    {#if current}<span class="current-body-pin" title={t('exploration-current-location')}>📍</span>{/if}{body.short_name}
                  </span>
                  <span role="cell" data-testid="body-kind" title={t(`exploration-kind-${body.kind_key}`)}>{t(`exploration-kind-${body.kind_key}`)}{body.star_class ? ` (${body.star_class})` : ''}</span>
                  <span role="cell" class="number">{num1(body.distance_ls)}</span>
                  <span role="cell" class="number font-mono">{credits(body.scanned_value)}</span>
                  <span role="cell" class="number font-mono" data-testid="mapped-value">{body.kind_key.startsWith('star-') ? '—' : credits(body.mapped_value)}</span>
                  <span role="cell">{t('exploration-signals-bio')} {body.bio ?? 0} · {t('exploration-signals-geo')} {body.geo ?? 0}</span>
                  <span role="cell" data-testid="body-marks" title={marks(body)}>{marks(body)}</span>
                  <span role="cell" class="advice-cell" title={advice(body) + (body.probes ? ' · ' + t(body.probes.estimated ? 'exploration-probes-estimated' : 'exploration-probes-exact', { count: body.probes.n }) : '')}>
                    {#if body.advice?.includes('map')}<span class="map-star">★ </span>{/if}{advice(body)}{#if body.probes} · {t(body.probes.estimated ? 'exploration-probes-estimated' : 'exploration-probes-exact', { count: body.probes.n })}{/if}
                  </span>
                </div>
              {/snippet}
            </VirtualList>
          </div>
        </div>
        {#if hasHorizontalOverflow && rowCount > 0}
          <div class="sticky-scrollbar" bind:this={stickyScrollEl} onscroll={onStickyScroll} aria-hidden="true" data-testid="sticky-horizontal-scrollbar">
            <div class="sticky-scrollbar-track" style:width="{tableScrollWidth}px"></div>
          </div>
        {/if}
      </div>
    {/if}
  {/snippet}
  <footer data-testid="exploration-footer">
    {#if system}<span>{t('exploration-unidentified')}: {system.unidentified ?? '—'}</span>{/if}
    {#if unsold}
      <div class="unsold-info">
        <span>{t('exploration-unsold')}: <strong class="gold font-mono">{credits(unsold.total)}</strong> · {t('exploration-unsold-systems', { count: unsold.systems })}</span>
        {#if unsold.at_risk}<strong class="badge badge-danger risk">{t('exploration-at-risk')}</strong>{/if}
      </div>
    {/if}
  </footer>
</Panel>

<style>
  .summary { display: flex; flex-wrap: wrap; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  .system-meta-col { display: flex; flex-direction: column; gap: 0.25rem; }
  .system-header { display: flex; align-items: center; gap: 0.5rem; }
  .system { font-size: var(--font-title-lg); color: var(--hud-primary); }
  .copy-btn {
    background: transparent;
    border: none;
    color: var(--hud-muted);
    cursor: pointer;
    padding: 2px 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    transition: color 150ms ease, background-color 150ms ease;
  }
  .copy-btn:hover {
    color: var(--hud-primary);
    background: rgba(255, 113, 0, 0.1);
  }
  .system-sub { display: flex; align-items: center; gap: 0.4rem; font-size: 0.75rem; color: var(--hud-muted); }
  .sub-pill {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
  }
  .home-pill {
    color: var(--hud-cyan, #38bdf8);
    border-color: rgba(56, 189, 248, 0.4);
    background: rgba(56, 189, 248, 0.08);
    font-weight: 600;
  }
  .home-dist-pill {
    color: var(--hud-cyan, #38bdf8);
    font-weight: 600;
  }
  .mono { font-family: var(--font-mono); }

  .stars { display: flex; align-items: center; gap: .4rem; }
  .stars-label { color: var(--hud-muted); font-size: 0.8rem; }
  .star-list { display: flex; align-items: center; gap: .3rem; }
  .star-badge { border: 1px solid var(--hud-border-subtle); background: var(--hud-surface-card); border-radius: 3px; padding: .15rem .45rem; font-family: var(--font-mono); font-size: .82em; font-weight: 700; color: var(--hud-muted); }
  .star-badge.scoopable { color: var(--hud-success); border-color: rgba(34, 197, 94, 0.4); background: rgba(34, 197, 94, 0.1); }
  
  .telemetry-chips { display: flex; flex-wrap: wrap; align-items: center; gap: 0.35rem; }
  .telemetry-chip {
    font-size: 0.72rem;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border-subtle);
    background: var(--hud-surface-card);
    font-weight: 500;
    color: var(--hud-muted);
  }
  .telemetry-chip.terraformable {
    border-color: rgba(56, 189, 248, 0.4);
    background: rgba(56, 189, 248, 0.1);
    color: #38bdf8;
  }
  .telemetry-chip.landable {
    border-color: rgba(234, 179, 8, 0.4);
    background: rgba(234, 179, 8, 0.1);
    color: #eab308;
  }
  .telemetry-chip.bio {
    border-color: rgba(34, 197, 94, 0.4);
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
  }
  .telemetry-chip.geo {
    border-color: rgba(249, 115, 22, 0.4);
    background: rgba(249, 115, 22, 0.1);
    color: #f97316;
  }
  .telemetry-chip.signals {
    border-color: rgba(168, 85, 247, 0.4);
    background: rgba(168, 85, 247, 0.1);
    color: #a855f7;
  }

  .fss-card { background: var(--hud-surface-card); border: 1px solid var(--hud-border-subtle); border-radius: 4px; padding: 0.45rem 0.75rem; display: flex; flex-direction: column; gap: 0.2rem; min-width: 14rem; }
  .fss-label-row { display: flex; justify-content: space-between; align-items: baseline; font-size: 0.75rem; }
  .fss-title { color: var(--hud-muted); font-weight: 600; text-transform: uppercase; font-size: 0.7rem; }
  .fss-card progress { width: 100%; height: 6px; accent-color: var(--hud-cyan); }
  .fss-count { color: var(--hud-muted); font-size: 0.72rem; }

  .value-card { min-width: 130px; }
  .empty { color: var(--hud-muted); }
  .number { font-family: var(--font-mono); font-variant-numeric: tabular-nums; text-align: right; }
  .font-mono { font-family: var(--font-mono); }
  .gold { color: #fbbf24; }
  .metric-sub { font-size: 0.72rem; color: var(--hud-muted); margin-top: 0.2rem; font-family: var(--font-mono); white-space: nowrap; }
  .metric-sub.green { color: var(--hud-success); font-weight: 600; }
  .text-dim { color: var(--hud-muted); opacity: 0.85; }
  
  .table-container { position: relative; display: flex; flex-direction: column; min-height: 0; }
  .table-scroll { overflow-x: auto; scrollbar-width: none; }
  .table-scroll::-webkit-scrollbar { display: none; }
  .table-scroll.no-overflow { overflow-x: hidden; }
  .body-table { min-width: 1350px; }
  .body-grid { display: grid; grid-template-columns: minmax(75px,.6fr) minmax(150px,1.4fr) minmax(85px,.7fr) minmax(110px,.9fr) minmax(110px,.9fr) minmax(110px,1fr) minmax(180px,1.5fr) minmax(210px,1.8fr); align-items: center; gap: .6rem; padding: 0 .4rem; }
  .headings { min-height: 2.5rem; color: var(--hud-primary); border-bottom: 1px solid var(--hud-border); font-size: 0.78rem; font-weight: 700; text-transform: uppercase; }
  .body-row { height: 30px; box-sizing: border-box; border-bottom: 1px solid var(--hud-border-subtle); transition: background-color 100ms ease; }
  .body-row:hover { background: rgba(255, 255, 255, 0.03); }
  .body-row span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .cell-body-name { font-weight: 600; color: #fff; display: flex; align-items: center; }
  .current-body-pin { display: inline-block; margin-right: 0.25rem; font-size: 0.75rem; }
  .body-row.current-body-row { background: rgba(56, 189, 248, 0.08); border-left: 3px solid #38bdf8; }
  .map-advice { background: rgba(255, 113, 0, 0.06); border-left: 3px solid var(--hud-primary); }
  .map-star { color: var(--hud-primary); font-weight: 700; }
  
  .sticky-scrollbar {
    position: sticky;
    bottom: 0;
    left: 0;
    right: 0;
    width: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--hud-surface);
    border-top: 1px solid var(--hud-border-subtle);
    z-index: 10;
    height: 12px;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.15);
  }
  .sticky-scrollbar::-webkit-scrollbar {
    height: 8px;
  }
  .sticky-scrollbar::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.25);
  }
  .sticky-scrollbar::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.5);
    border-radius: 4px;
  }
  .sticky-scrollbar::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }
  .sticky-scrollbar-track {
    height: 1px;
  }
  
  footer { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: .75rem; margin-top: .75rem; padding-top: 0.5rem; border-top: 1px solid var(--hud-border-subtle); font-size: 0.82rem; color: var(--hud-muted); }
  .unsold-info { display: flex; align-items: center; gap: 0.6rem; }
  .risk { color: var(--hud-danger); }
</style>
