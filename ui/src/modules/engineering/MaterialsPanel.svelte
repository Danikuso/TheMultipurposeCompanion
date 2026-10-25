<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { integer } from '../../lib/format';
  import { live, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { InventoryTopic, LockerTopic, LockerRow, LockerSection, MaterialCategory, SessionTopic } from './types';
  import { materialName, originLabel } from './labels';
  import { createPageReader, pageKey } from './pages.svelte';

  const categories: MaterialCategory[] = ['Raw', 'Manufactured', 'Encoded'];
  const categoryKeys = { Raw: 'raw', Manufactured: 'manufactured', Encoded: 'encoded' };
  const sections: LockerSection[] = ['items', 'components', 'consumables', 'data'];

  let tab = $state<MaterialCategory | 'locker'>('Raw');
  let section = $state<LockerSection>('components');
  let searchQuery = $state('');
  let visible = $state(!document.hidden);
  let showSession = $state(false);
  let collapsedGrades = $state<Record<string, boolean>>({});

  function toggleGrade(grade: string) {
    collapsedGrades[grade] = !collapsedGrades[grade];
  }

  const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
  const session = $derived(topic<SessionTopic>('engineering.session'));
  const revision = $derived(topic<LockerTopic>('engineering.locker')?.revision);
  const commander = $derived(topic<{ name?: string }>('core.cmdr')?.name ?? '');
  const reader = createPageReader<LockerRow>();

  const request = $derived(revision === undefined ? null : { resource: 'locker' as const, section, revision, commander });
  const currentKey = $derived(request ? pageKey(request) : null);
  const canRead = $derived(tab === 'locker' && visible && live.connected && request !== null);

  const rawGrades = $derived(tab === 'locker' ? [] : Object.entries(inventory?.by_category[tab] ?? {}).sort(([a], [b]) => Number(a) - Number(b)));

  // Filter materials based on search query
  const grades = $derived(
    rawGrades.map(([grade, rows]) => {
      if (!searchQuery.trim()) return [grade, rows] as const;
      const q = searchQuery.trim().toLowerCase();
      const filtered = rows.filter((row) => {
        const name = materialName(row).toLowerCase();
        return name.includes(q) || row.key.toLowerCase().includes(q);
      });
      return [grade, filtered] as const;
    }).filter(([_, rows]) => rows.length > 0)
  );

  // Category summary totals
  const categorySummary = $derived.by(() => {
    if (tab === 'locker') return null;
    let have = 0;
    let max = 0;
    for (const [grade, rows] of rawGrades) {
      const cap = inventory?.caps[grade] ?? 0;
      for (const row of rows) {
        have += row.count;
        max += cap;
      }
    }
    const pct = max > 0 ? Math.min(100, Math.round((have / max) * 100)) : 0;
    return { have, max, pct };
  });

  $effect(() => {
    const active = canRead;
    const next = request;
    if (active && next) void reader.load(next);
    else reader.clear();
    return () => reader.clear();
  });

  onMount(() => {
    const changed = () => { visible = !document.hidden; };
    document.addEventListener('visibilitychange', changed);
    return () => {
      document.removeEventListener('visibilitychange', changed);
      reader.clear();
    };
  });
</script>

<Panel titleKey="panel-engineering-materials" span={2} collapsible={true} defaultExpanded={false}>
  <div class="tabs-header">
    <div class="tabs" role="tablist" aria-label={t('panel-engineering-materials')}>
      {#each categories as category}
        <button
          role="tab"
          aria-selected={tab === category}
          data-testid="tab-{category}"
          onclick={() => { tab = category; searchQuery = ''; }}
        >
          {t(`engineering-tab-${categoryKeys[category]}`)}
        </button>
      {/each}
      <button
        role="tab"
        aria-selected={tab === 'locker'}
        data-testid="tab-locker"
        onclick={() => { tab = 'locker'; searchQuery = ''; }}
      >
        {t('engineering-tab-locker')}
      </button>
    </div>

    {#if categorySummary}
      <div class="category-gauge" title="{categorySummary.have} / {categorySummary.max}">
        <span class="gauge-label">{t('engineering-category-capacity', { have: integer(categorySummary.have), max: integer(categorySummary.max) })}</span>
        <div class="gauge-track">
          <div class="gauge-fill" style="width: {categorySummary.pct}%"></div>
        </div>
      </div>
    {/if}
  </div>

  {#if tab === 'locker'}
    <div class="locker-toolbar">
      <label>
        {t('engineering-locker-section')}
        <select bind:value={section} data-testid="locker-section">
          {#each sections as entry}
            <option value={entry}>{t(`engineering-locker-${entry}`)}</option>
          {/each}
        </select>
      </label>
    </div>
  {:else}
    <!-- Search Bar -->
    <div class="search-bar">
      <input
        type="search"
        bind:value={searchQuery}
        placeholder={t('engineering-search-placeholder')}
        class="search-input"
      />
    </div>
  {/if}

  <div class="session-section">
    <div class="session-header-row">
      <h3>{t('engineering-session-changes')}</h3>
      <button
        type="button"
        class="glass-btn-secondary list-toggle-btn"
        data-testid="toggle-session-section"
        onclick={() => (showSession = !showSession)}
      >
        {showSession ? '▲ Ocultar' : `▼ Ver (${Object.keys(session?.totals ?? {}).length})`}
      </button>
    </div>
    <ul class="session-list" class:is-collapsed={!showSession}>
      {#each Object.entries(session?.totals ?? {}) as [origin, counts] (origin)}
        <li data-origin={origin} class="session-item">
          <span>{originLabel(origin)}</span>
          <span class="mono session-counts">+{integer(counts.in)} / −{integer(counts.out)}</span>
        </li>
      {/each}
    </ul>
  </div>

  {#snippet details()}
    {#if tab === 'locker'}
      {#if canRead && reader.key === currentKey && reader.state === 'ready'}
        {#if reader.items.length === 0}
          <p class="muted">{t('engineering-no-data')}</p>
        {/if}
        <ul class="material-list">
          {#each reader.items as row (row.key)}
            <li data-key={row.key} class="material-item">
              <span class="material-name">{row.name_localised ?? row.key}</span>
              <span class="mono count-badge">{integer(row.count)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted" role="status">
          {t(reader.key === currentKey && reader.state === 'failed'
            ? 'engineering-page-failed'
            : reader.key === currentKey && reader.state === 'stale'
              ? 'engineering-page-stale'
              : canRead
                ? 'engineering-page-loading'
                : 'engineering-no-data')}
        </p>
      {/if}
    {:else}
      {#if grades.length === 0}
        <p class="muted">{t('engineering-no-data')}</p>
      {/if}

      {#each grades as [grade, rows] (grade)}
        {@const isCollapsed = collapsedGrades[grade] ?? false}
        <div class="grade-section">
          <h3 class="grade-header">
            <button
              type="button"
              class="grade-toggle-btn"
              data-testid={`toggle-grade-${grade}`}
              onclick={() => toggleGrade(grade)}
              aria-expanded={!isCollapsed}
            >
              <span class="grade-pill grade-pill-{grade}">G{grade}</span>
              <span>{Number(grade) > 0 ? t('engineering-grade', { grade: integer(Number(grade)) }) : t('engineering-no-data')}</span>
              <span class="fold-arrow">{isCollapsed ? '▼' : '▲'}</span>
            </button>
          </h3>
          <ul class="material-list" class:is-collapsed={isCollapsed}>
            {#each rows as row (row.key)}
              {@const maxCap = inventory?.caps[grade] ?? 0}
              {@const isAtMax = inventory?.at_max.includes(row.key)}
              {@const pct = maxCap > 0 ? Math.min(100, Math.round((row.count / maxCap) * 100)) : 0}
              <li data-key={row.key} class="material-item" class:item-at-max={isAtMax}>
                <div class="item-info">
                  <span class="material-name">
                    {materialName(row)}
                    {#if row.known === false || grade === '0'}
                      <small class="muted">{t('engineering-no-data')}</small>
                    {/if}
                  </span>
                  <div class="progress-wrap">
                    <div class="progress-bar">
                      <div class="progress-fill grade-{grade}" style="width: {pct}%"></div>
                    </div>
                  </div>
                </div>

                <div class="item-counts">
                  <span class="mono count-display">
                    {inventory?.caps[grade] ? t('engineering-count-of-max', { have: integer(row.count), max: integer(inventory.caps[grade]) }) : integer(row.count)}
                  </span>
                  {#if isAtMax}
                    <span class="badge-max">{t('engineering-at-max-badge')}</span>
                    <strong class="warning">{t('engineering-at-max')}</strong>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    {/if}
  {/snippet}
</Panel>

<style>
  .tabs-header { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: .6rem; margin-bottom: .8rem; }
  .tabs { display: flex; flex-wrap: wrap; gap: .4rem; }
  .grade-toggle-btn { background: none; border: none; padding: 0; color: inherit; display: flex; align-items: center; gap: 0.5rem; cursor: pointer; font: inherit; }
  .fold-arrow { font-size: 0.75rem; color: var(--hud-muted); }
  .material-list.is-collapsed { display: none; }
  .session-header-row { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; }
  .session-list.is-collapsed { display: none; }
  .list-toggle-btn { background: rgba(15, 23, 42, 0.6); border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.25)); color: var(--hud-text); padding: 0.2rem 0.5rem; font-size: 0.75rem; border-radius: 4px; cursor: pointer; }
  .list-toggle-btn:hover { border-color: var(--hud-primary); color: var(--hud-primary); }
  button, select, .search-input { color: var(--hud-text); background: var(--hud-bg); border: 1px solid var(--hud-muted); padding: .3rem .6rem; border-radius: 4px; font-size: .85rem; }
  button[aria-selected='true'] { border-color: var(--hud-primary); color: var(--hud-primary); background: rgba(255, 113, 0, 0.12); font-weight: 600; }
  
  .category-gauge { display: flex; align-items: center; gap: .5rem; font-size: .8rem; color: var(--hud-muted); }
  .gauge-track { width: 80px; height: 6px; background: rgba(255, 255, 255, 0.08); border-radius: 3px; overflow: hidden; border: 1px solid var(--hud-border); }
  .gauge-fill { height: 100%; background: var(--hud-accent, #00d2ff); transition: width .2s ease; }

  .search-bar { margin-bottom: .8rem; }
  .search-input { width: 100%; box-sizing: border-box; }
  .search-input:focus { border-color: var(--hud-primary); outline: none; }

  .grade-section { margin-bottom: 1rem; }
  .grade-header { display: flex; align-items: center; gap: .5rem; font-size: .95rem; color: var(--hud-primary); margin: .8rem 0 .4rem; }
  .grade-pill { font-size: .7rem; font-weight: bold; padding: .15rem .4rem; border-radius: 3px; background: rgba(255, 255, 255, 0.1); }
  .grade-pill-1 { background: rgba(160, 160, 160, 0.2); color: #ccc; }
  .grade-pill-2 { background: rgba(0, 160, 255, 0.2); color: #66c2ff; }
  .grade-pill-3 { background: rgba(180, 80, 255, 0.2); color: #d084ff; }
  .grade-pill-4 { background: rgba(255, 200, 0, 0.2); color: #ffd633; }
  .grade-pill-5 { background: rgba(255, 113, 0, 0.25); color: #ff944d; }

  .material-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: .35rem; }
  .material-item { display: flex; justify-content: space-between; align-items: center; gap: .8rem; padding: .35rem .5rem; border-radius: 4px; background: rgba(255, 255, 255, 0.02); border: 1px solid rgba(255, 255, 255, 0.05); }
  .material-item:hover { border-color: rgba(255, 113, 0, 0.4); background: rgba(255, 113, 0, 0.04); }
  .item-at-max { border-color: rgba(255, 80, 80, 0.4); background: rgba(255, 80, 80, 0.04); }

  .item-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: .25rem; }
  .material-name { font-size: .88rem; font-weight: 500; color: var(--hud-text); overflow-wrap: anywhere; }

  .progress-wrap { width: 100%; max-width: 220px; }
  .progress-bar { height: 4px; background: rgba(255, 255, 255, 0.08); border-radius: 2px; overflow: hidden; }
  .progress-fill { height: 100%; transition: width .2s; }
  .progress-fill.grade-1 { background: #999; }
  .progress-fill.grade-2 { background: #33a6ff; }
  .progress-fill.grade-3 { background: #b84dff; }
  .progress-fill.grade-4 { background: #ffcc00; }
  .progress-fill.grade-5 { background: #ff7100; }

  .item-counts { display: flex; flex-direction: column; align-items: flex-end; gap: .2rem; white-space: nowrap; }
  .count-display { font-size: .85rem; font-weight: 600; }
  .badge-max { font-size: .68rem; font-weight: bold; padding: .1rem .35rem; border-radius: 3px; background: rgba(255, 50, 50, 0.25); color: #ff6666; border: 1px solid rgba(255, 50, 50, 0.4); }

  .mono { font-family: var(--font-mono); }
  .muted { color: var(--hud-muted); }
  .warning { color: var(--hud-warning); font-size: .8rem; }

  .session-section { margin-top: 1.2rem; border-top: 1px solid var(--hud-border); padding-top: .6rem; }
  .session-section h3 { font-size: .92rem; color: var(--hud-primary); margin: .4rem 0 .4rem; }
  .session-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: .2rem; }
  .session-item { display: flex; justify-content: space-between; padding: .2rem 0; font-size: .85rem; }
  .session-counts { font-weight: 600; }
</style>
