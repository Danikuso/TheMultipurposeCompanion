<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blueprintLabel, contributionCommodityLabel, engineerStatusLabel, materialName } from './labels';
  import type { EngineerRow, EngineersTopic, InventoryTopic } from './types';

  const data = $derived(topic<EngineersTopic>('engineering.engineers'));
  const selected = $derived(data?.selected ?? null);
  const next = $derived(selected?.next_step ?? null);
  const rows = $derived(data?.engineers ?? []);
  const selectedRow = $derived(rows.find((row) => row.id === selected?.id));
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
</script>

<Panel titleKey="panel-engineering-engineers" span={2}>
  {#if failed}
    <p class="error" role="alert">{t('engineering-select-engineer-failed')}</p>
  {/if}
  {#if rows.length === 0}
    <p class="muted">{t('engineering-no-data')}</p>
  {/if}

  {#each [false, true] as onFoot}
    <section data-testid={onFoot ? 'engineers-on-foot' : 'engineers-ship'}>
      <h3>{t(onFoot ? 'engineering-engineer-on-foot' : 'engineering-engineer-ship')}</h3>
      <ul class="engineers">
        {#each rows.filter((row) => row.on_foot === onFoot) as row (row.id)}
          <li data-engineer-id={row.id} class:selected={selected?.id === row.id}>
            <button onclick={() => selectEngineer(row.id)} aria-pressed={selected?.id === row.id}>
              {row.name}
            </button>
            <span>{engineerStatusLabel(row.status)}</span>
            <span>{t('engineering-col-rank')}: {row.rank == null ? t('engineering-no-data') : integer(row.rank)}</span>
            <span>{t('engineering-engineer-rank-progress', {
              progress: row.rank_progress == null ? t('engineering-no-data') : `${integer(row.rank_progress)} %`,
            })}</span>
            <span>{row.system ?? t('engineering-no-data')}</span>
            <span class="mono">{distance(row)}</span>
            {#if !row.verified}<small class="muted">{t('engineering-unverified')}</small>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  {#if selected && selectedRow}
    <section class="details" data-testid="engineer-details">
      <h3>{selectedRow.name}</h3>
      <h4>{t('engineering-engineer-next-step')}</h4>
      <p data-testid="engineer-next-step">
        {#if next === 'Done'}
          {t('engineering-engineer-unlocked')}
        {:else if next && typeof next === 'object' && 'Requirement' in next}
          {requirement(next.Requirement.key)}
        {:else if next && typeof next === 'object' && 'Invite' in next}
          {requirement(next.Invite.key)}
        {:else if next && typeof next === 'object' && 'Contribute' in next}
          {@const contribution = contributionItem(next.Contribute.item, next.Contribute.kind)}
          {contribution.name}:
          {t('engineering-engineer-progress', {
            done: integer(next.Contribute.have), need: integer(next.Contribute.need),
          })}
          {#if contribution.pending}<small class="muted">{t('engineering-translation-pending')}</small>{/if}
        {:else}
          {t('engineering-no-data')}
        {/if}
      </p>
      <ul data-testid="engineer-blueprints">
        {#each selected.blueprints as entry (entry.blueprint)}
          {@const label = blueprintLabel(entry.blueprint, entry.name_en)}
          <li>
            {t('engineering-engineer-max-grade', {
              blueprint: label.name, grade: integer(entry.max_grade),
            })}
            {#if label.pending}<small class="muted">{t('engineering-translation-pending')}</small>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</Panel>

<style>
  h3 { color: var(--hud-primary); font-size: .95rem; margin: .8rem 0 .4rem; }
  h4 { font-size: .9rem; margin-bottom: .4rem; }
  ul { list-style: none; padding: 0; }
  .engineers li { display: flex; flex-wrap: wrap; align-items: center; gap: .3rem .8rem; padding: .5rem; border-bottom: 1px solid var(--hud-border); overflow-wrap: anywhere; }
  button { color: var(--hud-primary); background: var(--hud-bg); border: 1px solid var(--hud-border); padding: .3rem .5rem; text-align: left; }
  .selected { background: var(--hud-surface-alt); }
  .details { border-top: 1px solid var(--hud-primary); }
  .mono { font-family: var(--font-mono); }
  .muted { color: var(--hud-muted); }
  .error { color: var(--hud-danger); }
  small { display: inline-block; margin-left: .3rem; }
</style>
