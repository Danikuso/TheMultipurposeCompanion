<script lang="ts">
  import { t } from '../../lib/i18n';
  import { decimal, integer } from '../../lib/format';
  import { action, live, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { engineerStatusLabel } from './labels';
  import { createSectionsReader } from './sections.svelte';
  import { buildGuideSteps, guideAvailability, guideCargoStock, guideRank, type GuideShipCargo, type GuideTradeCargo } from './engineerGuide';
  import type { EngineerRows, EngineersTopic, InventoryTopic } from './types';

  const data = $derived(topic<EngineersTopic>('engineering.engineers'));
  const selectedId = $derived(data?.selected_id ?? null);
  const commander = $derived(topic<{ name?: string; credits?: number | null }>('core.cmdr'));
  const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
  const ship = $derived(topic<GuideShipCargo>('core.ship'));
  const tradeCargo = $derived(topic<GuideTradeCargo>('trade.cargo'));
  let retry = $state(0);
  let failedSelection = $state(false);
  let selecting = $state(false);
  let copyFeedback = $state<'copied' | 'failed' | null>(null);
  let selectionGeneration = 0;

  const list = createSectionsReader<EngineerRows>('engineers', ['engineers'], () => ({
    revision: data?.revision, commander: commander?.name ?? '', enabled: true, scope: String(retry),
  }));
  const rows = $derived(list.ready ? list.engineers.items : []);
  const selected = $derived(rows.find(row => row.id === selectedId));
  const guideReader = createSectionsReader<EngineerRows>('engineers', ['unlock'], () => ({
    revision: data?.revision, commander: commander?.name ?? '', enabled: selected !== undefined,
    engineerId: selectedId ?? undefined, scope: String(retry),
  }));
  const unlockRows = $derived(guideReader.ready ? guideReader.unlock.items.filter(row => row.engineer_id === selectedId) : []);
  const metadata = $derived(unlockRows.find(row => row.field === 'guide'));
  const contribution = $derived(unlockRows.find(row => row.field === 'contribution'));
  const prerequisites = $derived(unlockRows.filter(row => row.field === 'prerequisite'));
  const steps = $derived(buildGuideSteps(selected?.status ?? 'Unknown', unlockRows));
  const next = $derived(selected?.status === 'Acquainted' ? steps[2] : steps.find(step => step.state === 'current') ?? steps[0]);
  const rank = $derived(guideRank(selected?.rank ?? null, selected?.rank_progress ?? null));

  const available = $derived.by(() => {
    if (!contribution || !selected || selected.on_foot) return null;
    if (contribution.kind === 'credits') return commander?.credits ?? null;
    if (contribution.kind === 'commodity') {
      return guideCargoStock(contribution.item, ship, tradeCargo);
    }
    if (contribution.kind === 'material' && inventory?.snapshot) {
      for (const grades of Object.values(inventory.by_category)) for (const materials of Object.values(grades ?? {})) {
        const item = materials.find(row => row.key === contribution.item);
        if (item) return item.count;
      }
      return 0;
    }
    return null;
  });
  const stock = $derived(guideAvailability(available, contribution?.amount ?? 0));

  $effect(() => { selectedId; copyFeedback = null; });

  function instruction(key: string | null | undefined): string {
    if (!key) return t('engineering-no-data');
    const text = t(key);
    return text === key ? t('engineering-no-data') : text;
  }
  function loadMessage(state: string): string {
    if (!live.connected) return t('engineering-guide-disconnected');
    return t(state === 'failed' ? 'engineering-page-failed' : state === 'stale' ? 'engineering-data-stale' : state === 'loading' ? 'engineering-page-loading' : 'engineering-no-data');
  }
  async function selectEngineer(id: number | null) {
    // The shared action toggles a repeated id; a select control expresses a target instead.
    if (id === selectedId) return;
    const generation = ++selectionGeneration;
    failedSelection = false; selecting = true;
    try { await action('engineering.select_engineer', { id }); }
    catch { if (generation === selectionGeneration) failedSelection = true; }
    finally { if (generation === selectionGeneration) selecting = false; }
  }
  async function copySystem() {
    if (!selected?.system) return;
    const id = selected.id;
    copyFeedback = null;
    try {
      if (!navigator.clipboard) throw new Error('clipboard unavailable');
      await navigator.clipboard.writeText(selected.system);
      if (id === selectedId) copyFeedback = 'copied';
    } catch { if (id === selectedId) copyFeedback = 'failed'; }
  }
</script>

<Panel titleKey="panel-engineering-guide" span={2} class="engineer-guide-panel">
  <div class="guide" data-testid="engineer-guide">
    <label class="selector-label">
      <span>{t('engineering-guide-select')}</span>
      <select class="glass-select" data-testid="guide-select" value={selectedId ?? ''} disabled={!list.ready || selecting}
        onchange={(event) => {
          const value = event.currentTarget.value;
          event.currentTarget.value = selectedId === null ? '' : String(selectedId);
          void selectEngineer(value === '' ? null : Number(value));
        }}>
        <option value="">{t('engineering-guide-placeholder')}</option>
        {#each [false, true] as onFoot}
          <optgroup label={t(onFoot ? 'engineering-engineer-on-foot' : 'engineering-engineer-ship')}>
            {#each rows.filter(row => row.on_foot === onFoot) as row (row.id)}
              <option value={row.id}>{row.name} · {engineerStatusLabel(row.status)}</option>
            {/each}
          </optgroup>
        {/each}
      </select>
    </label>

    {#if failedSelection}<p class="error" role="alert">{t('engineering-select-engineer-failed')}</p>{/if}
    {#if !list.ready}
      <p class="muted" role="status">{loadMessage(list.state)}</p>
    {:else if selectedId === null}
      <p class="muted">{t('engineering-guide-intro')}</p>
    {:else if selected}
      <div class="identity">
        <div class="identity-heading">
          <h3>{selected.name}</h3>
          <span class="status-pill" class:success={selected.status === 'Unlocked'} class:danger={selected.status === 'Barred'}>{engineerStatusLabel(selected.status)}</span>
        </div>
        <div class="location-actions">
          <span class="location">{selected.system ?? t('engineering-no-data')}</span>
          {#if selected.distance_ly !== null && Number.isFinite(selected.distance_ly)}
            <span class="distance">{t('engineering-distance-ly', { distance: decimal(selected.distance_ly) })}</span>
          {/if}
          <button type="button" class="hud-btn" data-testid="guide-copy" disabled={!selected.system} onclick={copySystem}>{t('engineering-guide-copy')}</button>
        </div>
      </div>
      {#if copyFeedback}<p class:success={copyFeedback === 'copied'} class:error={copyFeedback === 'failed'} role="status">{t(copyFeedback === 'copied' ? 'engineering-guide-copied' : 'engineering-guide-copy-failed')}</p>{/if}

      {#if !guideReader.ready}
        <p class="muted" role="status">{loadMessage(guideReader.state)}</p>
      {:else}
        {#if !metadata?.verified}<span class="status-pill">{t('engineering-unverified')}</span>{/if}
        <ol class="stepper" aria-label={t('engineering-guide-stages')}>
          {#each steps as step, index (step.stage)}
            <li class="stage" class:confirmed={step.state === 'confirmed'} class:current={step.state === 'current'}
              data-stage={step.stage} data-state={step.state} aria-current={step.state === 'current' ? 'step' : undefined}>
              <span class="step-number" aria-hidden="true">{step.state === 'confirmed' ? '✓' : index + 1}</span>
              <span class="step-text"><strong>{t(`engineering-guide-stage-${step.stage}`)}</strong><small>{t(`engineering-guide-state-${step.state}`)}</small></span>
            </li>
          {/each}
        </ol>

        {#if selected.status === 'Barred'}<p class="error" role="status">{t('engineering-guide-barred')}</p>{/if}
        {#if selected.status === 'Acquainted'}<p class="muted">{t('engineering-guide-acquainted')}</p>{/if}
        {#if selected.status === 'Unlocked'}
          {#if !selected.on_foot}
            <div class="metric" data-testid="guide-rank">
              <span class="metric-label">{t('engineering-guide-rank')}</span>
              <div class="metric-row"><strong>{rank.rank === null ? t('engineering-no-data') : `${integer(rank.rank)} / 5`}</strong>
                <span>{rank.maximum ? t('engineering-guide-maximum') : rank.progress === null ? t('engineering-no-data') : t('engineering-guide-rank-progress', { progress: integer(rank.progress) })}</span></div>
              {#if rank.progress !== null}<div class="meter" role="progressbar" aria-label={t('engineering-guide-rank-progress-label')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={rank.progress}><span style:width={`${rank.progress}%`}></span></div>{/if}
            </div>
          {/if}
          <p class="orientation">{instruction(metadata?.reputation_key)}</p>
        {:else}
          <div class="next-action" data-testid="guide-next"><span class="metric-label">{t('engineering-engineer-next-step')}</span><p>{instruction(next.key)}</p></div>
          {#if contribution && selected.status !== 'Barred'}
            <div class="metric" data-testid="guide-stock">
              <span class="metric-label">{t('engineering-guide-stock')}</span>
              <div class="metric-row"><strong>{stock.available === null ? t('engineering-no-data') : integer(stock.available)} / {integer(stock.required)}</strong><span class:success={stock.missing === 0} class:shortage={stock.missing !== null && stock.missing > 0}>{t('engineering-guide-missing', { count: stock.missing === null ? t('engineering-no-data') : integer(stock.missing) })}</span></div>
              {#if stock.percent !== null}<div class="meter" class:ready={stock.missing === 0} role="progressbar" aria-label={t('engineering-guide-stock')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={stock.percent}><span style:width={`${stock.percent}%`}></span></div>{/if}
              <small class="muted">{t('engineering-guide-not-delivered')}</small>
            </div>
          {/if}
        {/if}
      {/if}
    {:else}
      <p class="muted">{t('engineering-no-data')}</p>
    {/if}
    {#if list.state === 'failed' || list.state === 'stale' || guideReader.state === 'failed' || guideReader.state === 'stale'}
      <button type="button" class="hud-btn" data-testid="guide-retry" onclick={() => retry++}>{t('engineering-guide-retry')}</button>
    {/if}
  </div>

  {#snippet details()}
    {#if selected && guideReader.ready}
      <div class="guide-details">
        <h3>{t('engineering-guide-all-requirements')}</h3>
        <ol class="requirements">
          {#each steps as step (step.stage)}<li><strong>{t(`engineering-guide-stage-${step.stage}`)}</strong><p>{instruction(step.key)}</p></li>{/each}
        </ol>
        {#if prerequisites.length}
          <h3>{t('engineering-guide-contacts')}</h3>
          <ul class="contacts">
            {#each prerequisites as prerequisite (prerequisite.prerequisite_id)}
              {@const contact = rows.find(row => row.id === prerequisite.prerequisite_id)}
              <li><p>{instruction(prerequisite.description_key)}</p>
                {#if contact}<button type="button" class="hud-btn" disabled={selecting} data-prerequisite-id={contact.id} onclick={() => selectEngineer(contact.id)}>{t('engineering-guide-open-contact', { name: contact.name })}</button>{/if}
              </li>
            {/each}
          </ul>
        {/if}
        {#if selected.status !== 'Unlocked'}<h3>{t('engineering-guide-after-unlock')}</h3><p class="orientation">{instruction(metadata?.reputation_key)}</p>{/if}
      </div>
    {/if}
  {/snippet}
</Panel>

<style>
  :global(.engineer-guide-panel .hud-panel-header) { flex-wrap: wrap; }
  :global(.engineer-guide-panel .hud-panel-header h2) { min-width: 0; overflow-wrap: anywhere; }
  :global(.engineer-guide-panel .panel-header-toggle) { max-width: 100%; }
  :global(.engineer-guide-panel .panel-header-toggle:hover) { background: var(--hud-primary); color: var(--hud-bg); }
  .guide, .guide-details { display: flex; flex-direction: column; gap: .65rem; min-width: 0; line-height: 1.45; overflow-wrap: anywhere; }
  .guide { container-type: inline-size; }
  .guide p, .guide-details p { margin: 0; }
  .selector-label { display: flex; flex-direction: column; gap: .3rem; color: var(--hud-muted); font-size: .8rem; min-width: 0; }
  .glass-select { width: 100%; min-width: 0; max-width: 100%; padding: .5rem; color: var(--hud-text); background: var(--hud-surface-alt); border: 1px solid var(--hud-border); border-radius: 4px; }
  .identity { display: flex; flex-direction: column; gap: .35rem; min-width: 0; }
  .identity-heading, .location-actions, .metric-row { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: .35rem .65rem; min-width: 0; }
  h3 { margin: 0; font-size: .9rem; color: var(--hud-primary); }
  .identity-heading h3 { flex: 1 1 10rem; }
  .location { color: var(--hud-text); }
  .distance { color: var(--hud-cyan); font-family: var(--font-mono); font-size: .8rem; }
  .status-pill { display: inline-flex; max-width: 100%; border: 1px solid var(--hud-border); border-radius: 1rem; padding: .15rem .5rem; color: var(--hud-warning); font-size: .72rem; }
  .success, .confirmed { color: var(--hud-success); }
  .danger, .error { color: var(--hud-danger); }
  .shortage { color: var(--hud-warning); }
  .muted, small { color: var(--hud-muted); font-size: .78rem; }
  .hud-btn { color: var(--hud-text); background: var(--hud-surface-alt); border: 1px solid var(--hud-border); border-radius: 4px; padding: .4rem .65rem; cursor: pointer; max-width: 100%; min-width: 0; white-space: normal; overflow-wrap: anywhere; transition: border-color 150ms ease; }
  .hud-btn:hover:not(:disabled) { border-color: var(--hud-primary); color: var(--hud-primary); }
  .hud-btn:disabled { opacity: .5; cursor: default; }
  .hud-btn:focus-visible, select:focus-visible { outline: 2px solid var(--hud-cyan); outline-offset: 2px; }
  .stepper { display: flex; flex-wrap: wrap; gap: .4rem; list-style: none; padding: 0; margin: 0; }
  .stage { flex: 1 1 8rem; min-width: 0; display: flex; align-items: center; gap: .45rem; border: 1px solid var(--hud-border-subtle); background: var(--hud-surface-card); border-radius: 5px; padding: .45rem; }
  .stage.confirmed { border-color: var(--hud-success); }
  .stage.current { border-color: var(--hud-primary); }
  .step-number { display: grid; place-items: center; width: 1.5rem; height: 1.5rem; flex-shrink: 0; border: 1px solid currentColor; border-radius: 50%; font-family: var(--font-mono); font-size: .8rem; }
  .step-text { display: flex; flex-direction: column; min-width: 0; font-size: .75rem; }
  .step-text small { font-size: .65rem; }
  .next-action, .metric { background: var(--hud-surface-card); border: 1px solid var(--hud-border-subtle); border-radius: 5px; padding: .65rem; min-width: 0; }
  .next-action { border-left: 3px solid var(--hud-primary); }
  .next-action p, .orientation, .requirements p, .contacts p { font-size: .83rem; }
  .metric-label { color: var(--hud-muted); font-size: .68rem; text-transform: uppercase; letter-spacing: .04em; }
  .metric-row { margin: .2rem 0; font-size: .8rem; }
  .metric-row strong { color: var(--hud-cyan); font-family: var(--font-mono); font-size: 1.1rem; }
  .meter { height: .4rem; border-radius: .25rem; background: var(--hud-bg); overflow: hidden; margin: .4rem 0; }
  .meter span { display: block; height: 100%; background: var(--hud-warning); }
  .meter.ready span { background: var(--hud-success); }
  .requirements { padding-left: 1.3rem; margin: 0; display: flex; flex-direction: column; gap: .7rem; }
  .requirements strong { font-size: .8rem; color: var(--hud-cyan); }
  .contacts { display: flex; flex-direction: column; gap: .6rem; padding: 0; margin: 0; list-style: none; }
  .contacts li { display: flex; flex-direction: column; align-items: flex-start; gap: .4rem; border: 1px solid var(--hud-border-subtle); padding: .6rem; border-radius: 5px; min-width: 0; }
  @container (max-width: 180px) {
    .stage { flex-direction: column; align-items: flex-start; }
  }
</style>
