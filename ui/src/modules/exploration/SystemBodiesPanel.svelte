<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, num1, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import VirtualList from '../../shell/VirtualList.svelte';
  import type { BodyRow, SystemTopic, UnsoldTopic } from './types';

  const system = $derived(topic<SystemTopic>('exploration.system'));
  const unsold = $derived(topic<UnsoldTopic>('exploration.unsold'));
  const progress = $derived(system?.fss.complete ? 1 : Math.max(0, Math.min(1, system?.fss.progress ?? 0)));
  const columns = ['body', 'type', 'distance', 'scanned', 'mapped', 'signals', 'marks', 'advice'];
  const markKeys = ['terraformable', 'landable', 'scoopable', 'first-discovery', 'already-mapped', 'mapped-by-you'];
  function marks(body: BodyRow): string {
    return markKeys.filter((_, index) => ((body.marks ?? 0) & (1 << index)) !== 0)
      .map(key => t(`exploration-mark-${key}`)).join(' · ') || '—';
  }
  function advice(body: BodyRow): string {
    return (body.advice ?? ['not_worth']).map(key => t(`exploration-advice-${key.replaceAll('_', '-')}`)).join(' · ');
  }
</script>

<Panel titleKey="panel-system-bodies" span="full" size="large">
  {#if system}
    <div class="summary">
      <strong class="system" data-testid="system-name">{system.system}</strong>
      {#if system.discovered != null}
        <span class="badge">{t(system.discovered ? 'exploration-discovered' : 'exploration-undiscovered')}</span>
      {/if}
      <div class="fss">
        <label for="exploration-fss">{t('exploration-fss-progress')} {system.fss.complete ? pct(1) : pct(system.fss.progress)}</label>
        <progress id="exploration-fss" value={progress} max="1"></progress>
        <span>{t('exploration-scanned-count', { scanned: system.fss.scanned, total: system.fss.body_count ?? '—' })}</span>
      </div>
      <div class="value"><span>{t('exploration-value-now')}</span><strong>{credits(system.value_now)}</strong></div>
      <div class="value"><span>{t('exploration-value-if-mapped')}</span><strong>{credits(system.value_if_mapped)}</strong></div>
    </div>
    {#if (system.truncated ?? 0) > 0}
      <p data-testid="exploration-truncated">{t('exploration-truncated', { count: system.truncated ?? 0 })}</p>
    {/if}
    <div class="table-scroll">
      <div role="table" aria-label={t('panel-system-bodies')} aria-rowcount={system.rows.length + 1} class="body-table">
        <div class="body-grid headings" role="row">
          {#each columns as column}<span role="columnheader">{t(`exploration-col-${column}`)}</span>{/each}
        </div>
        <VirtualList items={system.rows} rowHeight={30} height={600} label={t('panel-system-bodies')}>
          {#snippet row(body, index)}
            <div role="row" aria-rowindex={index + 2} class="body-grid body-row" class:map-advice={body.advice?.includes('map')}
              data-testid="body-row" data-body-id={body.body_id}>
              <span role="cell" title={body.short_name}>{body.short_name}</span>
              <span role="cell" data-testid="body-kind" title={t(`exploration-kind-${body.kind_key}`)}>{t(`exploration-kind-${body.kind_key}`)}{body.star_class ? ` (${body.star_class})` : ''}</span>
              <span role="cell" class="number">{num1(body.distance_ls)}</span>
              <span role="cell" class="number">{credits(body.scanned_value)}</span>
              <span role="cell" class="number" data-testid="mapped-value">{body.kind_key.startsWith('star-') ? '—' : credits(body.mapped_value)}</span>
              <span role="cell">{t('exploration-signals-bio')} {body.bio ?? 0} · {t('exploration-signals-geo')} {body.geo ?? 0}</span>
              <span role="cell" data-testid="body-marks" title={marks(body)}>{marks(body)}</span>
              <span role="cell" title={advice(body) + (body.probes ? ' · ' + t(body.probes.estimated ? 'exploration-probes-estimated' : 'exploration-probes-exact', { count: body.probes.n }) : '')}>{body.advice?.includes('map') ? '★ ' : ''}{advice(body)}{#if body.probes} · {t(body.probes.estimated ? 'exploration-probes-estimated' : 'exploration-probes-exact', { count: body.probes.n })}{/if}</span>
            </div>
          {/snippet}
        </VirtualList>
      </div>
    </div>
  {:else}
    <p class="empty">{t('empty-no-data')}</p>
  {/if}
  <footer data-testid="exploration-footer">
    {#if system}<span>{t('exploration-unidentified')}: {system.unidentified ?? '—'}</span>{/if}
    {#if unsold}
      <span>{t('exploration-unsold')}: <strong>{credits(unsold.total)}</strong> · {t('exploration-unsold-systems', { count: unsold.systems })}</span>
      {#if unsold.at_risk}<strong class="risk">{t('exploration-at-risk')}</strong>{/if}
    {/if}
  </footer>
</Panel>

<style>
  .summary { display: flex; flex-wrap: wrap; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  .system { font-size: var(--font-title-lg); color: var(--hud-primary); }
  .badge { border: 1px solid var(--hud-border); border-radius: 3px; padding: .2rem .5rem; }
  .fss { display: grid; gap: .2rem; min-width: 12rem; }
  progress { width: 100%; accent-color: var(--hud-primary); }
  .value { display: grid; gap: .2rem; }
  .value span, .fss span, .empty { color: var(--hud-muted); }
  .value strong, .number { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .table-scroll { overflow-x: auto; }
  .body-table { min-width: 1350px; }
  .body-grid { display: grid; grid-template-columns: minmax(75px,.6fr) minmax(150px,1.4fr) minmax(85px,.7fr) minmax(110px,.9fr) minmax(110px,.9fr) minmax(110px,1fr) minmax(180px,1.5fr) minmax(210px,1.8fr); align-items: center; gap: .6rem; padding: 0 .4rem; }
  .headings { min-height: 2.5rem; color: var(--hud-primary); border-bottom: 1px solid var(--hud-border); }
  .body-row { height: 30px; box-sizing: border-box; border-bottom: 1px solid var(--hud-border); }
  .body-row span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .map-advice { background: var(--hud-surface-alt); border-left: 3px solid var(--hud-primary); }
  .number { text-align: right; }
  footer { display: flex; flex-wrap: wrap; justify-content: space-between; gap: .75rem; margin-top: .75rem; }
  .risk { color: var(--hud-danger); }
</style>
