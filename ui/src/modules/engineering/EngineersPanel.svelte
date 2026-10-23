<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blueprintLabel, contributionCommodityLabel, engineerStatusLabel, engineeringDataLabel, materialName } from './labels';
  import { createSectionsReader } from './sections.svelte';
  import type { EngineerRow, EngineerRows, EngineersTopic, InventoryTopic } from './types';

  const data = $derived(topic<EngineersTopic>('engineering.engineers'));
  const selectedId = $derived(data?.selected_id ?? null);
  const next = $derived(data?.next_step ?? null);
  const commander = $derived(topic<{ name?: string }>('core.cmdr')?.name ?? '');

  // Filter state
  let statusFilter = $state<'all' | 'unlocked' | 'pending'>('all');

  // Lectores privados
  const list = createSectionsReader<EngineerRows>('engineers', ['engineers'], () => ({ revision: data?.revision, commander, enabled: true }));

  const catalogueKey = $derived(JSON.stringify(topic<{ counts?: unknown }>('engineering.catalogue')?.counts ?? null));
  const caps = createSectionsReader<EngineerRows>('engineers', ['capabilities'], () => ({
    revision: data?.revision, commander, enabled: selectedId !== null, engineerId: selectedId ?? undefined, cacheKey: `${selectedId}|${catalogueKey}`,
  }));

  const rows = $derived(list.ready ? list.engineers.items : []);
  const selectedRow = $derived(rows.find((row) => row.id === selectedId));

  let lastSelected = $state<{ id: number; name: string } | null>(null);
  $effect(() => { if (selectedRow) lastSelected = { id: selectedRow.id, name: selectedRow.name }; });
  const selectedName = $derived(selectedRow?.name ?? (lastSelected?.id === selectedId ? lastSelected?.name : undefined));

  const capabilities = $derived.by(() => {
    if (!caps.ready || selectedId === null) return [];
    const grouped = new Map<string, { module: string; blueprint: string; grades: number[] }>();
    for (const row of caps.capabilities.items) {
      if (row.engineer_id !== selectedId) continue;
      const key = `${row.module_type}|${row.blueprint}`;
      const entry = grouped.get(key) ?? { module: row.module_type, blueprint: row.blueprint, grades: [] };
      if (!entry.grades.includes(row.grade)) entry.grades.push(row.grade);
      grouped.set(key, entry);
    }
    return [...grouped.values()].map((entry) => ({ ...entry, grades: entry.grades.sort((a, b) => a - b) }));
  });

  const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
  const shipCargo = $derived(topic<{
    cargo?: { Name: string; Name_Localised?: string | null; Count: number }[];
  }>('core.ship')?.cargo ?? []);
  const tradeCargo = $derived(topic<{
    lines?: { commodity: string; name_localised: string | null }[];
  }>('trade.cargo')?.lines ?? []);

  let failed = $state(false);
  let selectionGeneration = 0;

  async function selectEngineer(id: number) {
    const current = ++selectionGeneration;
    failed = false;
    try {
      await action('engineering.select_engineer', { id });
    } catch {
      if (current === selectionGeneration) failed = true;
    }
  }

  function distance(row: EngineerRow): string {
    return row.distance_ly != null && Number.isFinite(row.distance_ly)
      ? t('engineering-distance-ly', { distance: decimal(row.distance_ly) })
      : t('engineering-no-data');
  }

  function requirement(key: string): string {
    const translated = t(key);
    return translated === key ? t('engineering-no-data') : translated;
  }

  function contributionItem(key: string, kind: string): { name: string; pending: boolean } {
    if (kind === 'credits') return { name: t('engineering-contribution-credits'), pending: false };
    if (kind === 'commodity') {
      const journalName = shipCargo.find((row) => row.Name === key)?.Name_Localised
        || tradeCargo.find((row) => row.commodity === key)?.name_localised;
      return contributionCommodityLabel(key, journalName);
    }
    for (const grades of Object.values(inventory?.by_category ?? {})) {
      for (const materials of Object.values(grades)) {
        const row = materials.find((material) => material.key === key);
        if (row) return { name: materialName(row), pending: false };
      }
    }
    return { name: key, pending: false };
  }

  function matchesFilter(row: EngineerRow): boolean {
    if (statusFilter === 'unlocked') return row.status === 'Unlocked';
    if (statusFilter === 'pending') return row.status !== 'Unlocked';
    return true;
  }

  function statusClass(status: string): string {
    switch (status) {
      case 'Unlocked': return 'status-badge-unlocked';
      case 'Invited':
      case 'Acquainted': return 'status-badge-invited';
      case 'Known': return 'status-badge-known';
      default: return 'status-badge-barred';
    }
  }

  function rankStars(rank: number | null | undefined): string {
    if (rank == null || rank <= 0) return '';
    return '★'.repeat(Math.min(5, Math.max(1, rank)));
  }
</script>

<Panel titleKey="panel-engineering-engineers" span={2}>
  {#if failed}
    <p class="error" role="alert">{t('engineering-select-engineer-failed')}</p>
  {/if}

  {#if !list.ready}
    <p class="muted" role="status">
      {t(list.state === 'failed'
        ? 'engineering-page-failed'
        : list.state === 'stale'
          ? 'engineering-data-stale'
          : list.state === 'loading'
            ? 'engineering-page-loading'
            : 'engineering-no-data')}
    </p>
  {:else if rows.length === 0}
    <p class="muted">{t('engineering-no-data')}</p>
  {/if}

  <!-- Status Filter Tabs -->
  <div class="filter-tabs">
    <button
      class="filter-tab"
      class:active={statusFilter === 'all'}
      onclick={() => { statusFilter = 'all'; }}
    >
      {t('engineering-filter-all')} ({rows.length})
    </button>
    <button
      class="filter-tab"
      class:active={statusFilter === 'unlocked'}
      onclick={() => { statusFilter = 'unlocked'; }}
    >
      {t('engineering-filter-unlocked')} ({rows.filter((r) => r.status === 'Unlocked').length})
    </button>
    <button
      class="filter-tab"
      class:active={statusFilter === 'pending'}
      onclick={() => { statusFilter = 'pending'; }}
    >
      {t('engineering-filter-pending')} ({rows.filter((r) => r.status !== 'Unlocked').length})
    </button>
  </div>

  <div class="engineers-container">
    {#each [false, true] as onFoot}
      {@const filtered = rows.filter((row) => row.on_foot === onFoot && matchesFilter(row))}
      <section data-testid={onFoot ? 'engineers-on-foot' : 'engineers-ship'} class="engineers-section">
        <h3 class="section-title">{t(onFoot ? 'engineering-engineer-on-foot' : 'engineering-engineer-ship')}</h3>
        <ul class="engineers">
          {#each filtered as row (row.id)}
            <li data-engineer-id={row.id} class="engineer-card" class:selected={selectedId === row.id}>
              <div class="card-top">
                <button
                  class="engineer-button"
                  onclick={() => selectEngineer(row.id)}
                  aria-pressed={selectedId === row.id}
                >
                  <span class="engineer-name">{row.name}</span>
                </button>
                <span class="status-badge {statusClass(row.status)}">
                  {engineerStatusLabel(row.status)}
                </span>
              </div>

              <div class="card-details">
                <div class="rank-info">
                  <span class="rank-label">{t('engineering-col-rank')}: {row.rank == null ? t('engineering-no-data') : integer(row.rank)}</span>
                  {#if row.rank != null && row.rank > 0}
                    <span class="rank-stars" title={String(row.rank)}>{rankStars(row.rank)}</span>
                  {/if}
                  <span class="rank-progress">
                    {t('engineering-engineer-rank-progress', {
                      progress: row.rank_progress == null ? t('engineering-no-data') : `${integer(row.rank_progress)} %`,
                    })}
                  </span>
                </div>

                <div class="location-info">
                  <span class="system-name">{row.system ?? t('engineering-no-data')}</span>
                  <span class="mono distance-val">{distance(row)}</span>
                </div>
              </div>

              {#if !row.verified}
                <small class="muted unverified-tag">{t('engineering-unverified')}</small>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}

    {#if selectedId !== null && selectedName !== undefined}
      <section class="details" data-testid="engineer-details">
        <div class="details-header">
          <h3 class="details-name">{selectedName}</h3>
          {#if selectedRow}
            <span class="status-badge {statusClass(selectedRow.status)}">
              {engineerStatusLabel(selectedRow.status)}
            </span>
          {/if}
        </div>

        <div class="next-step-box">
          <h4 class="next-step-title">{t('engineering-engineer-next-step')}</h4>
          <p data-testid="engineer-next-step" class="next-step-desc">
            {#if next === 'Done'}
              <span class="done-mark">✔ {t('engineering-engineer-unlocked')}</span>
            {:else if next && typeof next === 'object' && 'Requirement' in next}
              {requirement(next.Requirement.key)}
            {:else if next && typeof next === 'object' && 'Invite' in next}
              {requirement(next.Invite.key)}
            {:else if next && typeof next === 'object' && 'Contribute' in next}
              {@const contribution = contributionItem(next.Contribute.item, next.Contribute.kind)}
              <span class="contribution-item">{contribution.name}:</span>
              <span class="contribution-progress">
                {t('engineering-engineer-progress', {
                  done: integer(next.Contribute.have), need: integer(next.Contribute.need),
                })}
              </span>
              {#if contribution.pending}
                <small class="muted">{t('engineering-translation-pending')}</small>
              {/if}
            {:else}
              {t('engineering-no-data')}
            {/if}
          </p>
        </div>

        <h4 class="caps-title">{t('engineering-engineer-capabilities')}</h4>
        {#if caps.ready}
          <ul data-testid="engineer-blueprints" class="caps-grid">
            {#each capabilities as entry (`${entry.module}/${entry.blueprint}`)}
              {@const label = blueprintLabel(entry.blueprint)}
              {@const module = engineeringDataLabel('modtype', entry.module)}
              <li class="caps-card">
                <div class="caps-text">
                  {t('engineering-engineer-capability', {
                    module: module.name, blueprint: label.name,
                    grades: entry.grades.map((grade) => t('engineering-grade', { grade: integer(grade) })).join(', '),
                  })}
                </div>
                {#if label.pending || module.pending}
                  <small class="muted">{t('engineering-translation-pending')}</small>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted" role="status">
            {t(caps.state === 'failed'
              ? 'engineering-page-failed'
              : caps.state === 'stale'
                ? 'engineering-data-stale'
                : 'engineering-page-loading')}
          </p>
        {/if}
      </section>
    {/if}
  </div>
</Panel>

<style>
  .filter-tabs { display: flex; gap: .4rem; margin-bottom: .8rem; }
  .filter-tab { color: var(--hud-text); background: var(--hud-bg); border: 1px solid var(--hud-muted); padding: .25rem .5rem; border-radius: 4px; font-size: .8rem; }
  .filter-tab.active { border-color: var(--hud-primary); color: var(--hud-primary); background: rgba(255, 113, 0, 0.12); font-weight: 600; }

  .engineers-container { display: flex; flex-direction: column; gap: 1rem; }
  .engineers-section { display: flex; flex-direction: column; gap: .4rem; }
  .section-title { color: var(--hud-primary); font-size: .95rem; margin: .4rem 0 .2rem; }

  ul { list-style: none; padding: 0; margin: 0; }
  .engineers { display: flex; flex-direction: column; gap: .4rem; }

  .engineer-card { display: flex; flex-direction: column; gap: .3rem; padding: .6rem; border: 1px solid var(--hud-border); border-radius: 6px; background: rgba(255, 255, 255, 0.02); transition: border-color .15s ease; }
  .engineer-card:hover { border-color: rgba(255, 113, 0, 0.4); }
  .engineer-card.selected { border-color: var(--hud-primary); background: rgba(255, 113, 0, 0.08); }

  .card-top { display: flex; justify-content: space-between; align-items: center; gap: .6rem; }
  .engineer-button { color: var(--hud-text); background: transparent; border: none; padding: 0; text-align: left; cursor: pointer; }
  .engineer-name { font-size: .95rem; font-weight: 600; color: var(--hud-primary); }
  .engineer-card:hover .engineer-name { text-decoration: underline; }

  .status-badge { font-size: .72rem; font-weight: bold; padding: .15rem .45rem; border-radius: 3px; white-space: nowrap; }
  .status-badge-unlocked { background: rgba(0, 255, 100, 0.15); color: #00ff66; border: 1px solid rgba(0, 255, 100, 0.3); }
  .status-badge-invited { background: rgba(255, 204, 0, 0.15); color: #ffcc00; border: 1px solid rgba(255, 204, 0, 0.3); }
  .status-badge-known { background: rgba(0, 210, 255, 0.15); color: #00d2ff; border: 1px solid rgba(0, 210, 255, 0.3); }
  .status-badge-barred { background: rgba(120, 120, 120, 0.15); color: #aaa; border: 1px solid rgba(120, 120, 120, 0.3); }

  .card-details { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: .4rem 1rem; font-size: .82rem; }
  .rank-info { display: flex; align-items: center; gap: .4rem; }
  .rank-stars { color: #ffcc00; font-size: .8rem; }
  .location-info { display: flex; align-items: center; gap: .6rem; }
  .distance-val { color: var(--hud-accent, #00d2ff); font-weight: 600; }
  .unverified-tag { font-size: .75rem; }

  .details { border-top: 1px solid var(--hud-primary); padding-top: .8rem; margin-top: .4rem; }
  .details-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: .6rem; }
  .details-name { font-size: 1.05rem; font-weight: 700; color: var(--hud-primary); margin: 0; }

  .next-step-box { background: rgba(255, 255, 255, 0.03); border: 1px solid var(--hud-border); border-radius: 5px; padding: .6rem; margin-bottom: .8rem; }
  .next-step-title { font-size: .85rem; font-weight: 600; color: var(--hud-text); margin: 0 0 .3rem; }
  .next-step-desc { font-size: .85rem; margin: 0; color: var(--hud-text); }
  .done-mark { color: #00ff66; font-weight: 600; }
  .contribution-item { font-weight: 600; margin-right: .3rem; }
  .contribution-progress { font-weight: 700; color: var(--hud-primary); }

  .caps-title { font-size: .88rem; color: var(--hud-text); margin: .6rem 0 .4rem; }
  .caps-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: .4rem; }
  .caps-card { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--hud-border); border-radius: 4px; padding: .4rem .6rem; font-size: .82rem; }

  .mono { font-family: var(--font-mono); }
  .muted { color: var(--hud-muted); }
  .error { color: var(--hud-danger); }
  small { display: inline-block; margin-left: .3rem; }
</style>
