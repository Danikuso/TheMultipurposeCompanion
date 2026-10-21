<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { integer, decimal } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import { engineeringActions, shipFlowAction, syncEngineeringContext } from './actions.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blueprintLabel, engineeringDataLabel, engineerStatusLabel, materialName, contributionCommodityLabel } from './labels';
  import type { EngineeringFlowTopic, InventoryTopic } from './types';

  const data = $derived(topic<EngineeringFlowTopic>('engineering.flow'));
  const slot = $derived(data?.slot ?? null);
  const identity = $derived(JSON.stringify([topic<{ name?: string }>('core.cmdr')?.name, topic<{ ship_id?: number }>('engineering.ship')?.ship_id]));
  const context = $derived(JSON.stringify([identity, slot]));
  const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
  const recommended = $derived(data?.engineer?.nearest_unlocked ?? data?.engineer?.nearest_any);
  const next = $derived(data?.engineer?.next_step);
  let defaults = $state([1, 1, 1, 1, 4]);
  let blueprint = $state('');
  let experimental = $state('');
  let grade = $state(5);
  let rolls = $state([1, 1, 1, 1, 4]);
  let initializedContext: string | undefined = undefined;
  let edited = false;
  let failed = $state(false);
  let busy = $state(false);
  let generation = 0;
  let pendingPreview = $state<string | null>(null);
  const draft = $derived(JSON.stringify({ blueprint, experimental: experimental || null, grade, rolls: [...rolls] }));
  const confirmed = $derived(data?.request ? JSON.stringify({ blueprint: data.request.blueprint, experimental: data.request.experimental, grade: data.request.target_grade, rolls: data.request.rolls }) : null);
  const awaitingPreview = $derived(pendingPreview !== null && pendingPreview !== confirmed);
  const calculated = $derived(confirmed === draft);
  const selectionPending = $derived(engineeringActions.selectedSlot !== null && engineeringActions.selectedSlot !== slot);
  const valid = $derived(blueprint !== '' && rolls.length === 5 && rolls.every(n => Number.isInteger(n) && n >= 1 && n <= 10));

  $effect(() => { if (engineeringActions.selectedSlot === slot) engineeringActions.selectedSlot = null; });
  $effect(() => { syncEngineeringContext(identity); });
  $effect(() => { if (pendingPreview === confirmed) pendingPreview = null; });

  $effect(() => {
    if (context === initializedContext) return;
    initializedContext = context;
    generation++;
    failed = false;
    busy = false;
    pendingPreview = null;
    edited = false;
    blueprint = data?.request?.blueprint ?? data?.options.blueprints[0] ?? '';
    experimental = data?.request?.experimental ?? '';
    grade = data?.request?.target_grade ?? 5;
    rolls = [...(data?.request?.rolls ?? defaults)];
  });

  onMount(() => {
    const controller = new AbortController();
    let active = true;
    void (async () => {
      try {
        const response = await fetch('/api/config', { signal: controller.signal });
        if (!response.ok) return;
        const cfg = await response.json();
        const saved = cfg.engineering_rolls;
        if (!active || !Array.isArray(saved) || saved.length !== 5 || !saved.every(n => Number.isInteger(n) && n >= 1 && n <= 10)) return;
        defaults = [...saved];
        if (!edited && !data?.request) rolls = [...saved];
      } catch { /* los valores de fábrica siguen disponibles sin configuración */ }
    })();
    return () => { active = false; controller.abort(); generation++; };
  });

  function edit() { edited = true; failed = false; }
  async function send(name: string, args: unknown) {
    if (busy || engineeringActions.pending || selectionPending) return;
    const current = ++generation;
    busy = true; failed = false;
    if (name === 'preview') pendingPreview = JSON.stringify(args);
    try { await shipFlowAction(name, args); }
    catch { if (current === generation) { failed = true; pendingPreview = null; } }
    finally { if (current === generation) busy = false; }
  }
  function name(key: string): string {
    for (const grades of Object.values(inventory?.by_category ?? {})) {
      for (const rows of Object.values(grades)) { const row = rows.find(row => row.key === key); if (row) return materialName(row); }
    }
    return data?.lines.find(row => row.key === key)?.name_en ?? key;
  }
  function requirement(key: string): string { const text = t(key); return text === key ? t('engineering-no-data') : text; }
</script>
<Panel titleKey="panel-engineering-flow" span={2}>
  {#if !slot}<p>{t('engineering-ship-pick-module')}</p>
  {:else}
    <p>{data?.module_type ? engineeringDataLabel('modtype', data.module_type).name : data?.item} · {slot}</p>
    {#if failed}<p role="alert">{t('engineering-action-failed')}</p>{/if}
    <div class="controls">
      <label>{t('engineering-col-blueprint')}<select data-testid="blueprint" bind:value={blueprint} onchange={edit}>
        {#each data?.options.blueprints ?? [] as key}{@const label = blueprintLabel(key)}<option value={key}>{label.name}{label.pending ? ` · ${t('engineering-translation-pending')}` : ''}</option>{/each}
      </select></label>
      <label>{t('engineering-col-experimental')}<select data-testid="experimental" bind:value={experimental} onchange={edit}>
        <option value="">{t('engineering-no-experimental')}</option>
        {#each data?.options.experimentals ?? [] as key}{@const label = engineeringDataLabel('exp', key)}<option value={key}>{label.name}{label.pending ? ` · ${t('engineering-translation-pending')}` : ''}</option>{/each}
      </select></label>
      <label>{t('engineering-target-grade')}<select data-testid="target-grade" bind:value={grade} onchange={edit}>{#each [1,2,3,4,5] as value}<option {value}>{t('engineering-grade', { grade: integer(value) })}</option>{/each}</select></label>
    </div>
    <fieldset><legend>{t('engineering-rolls')}</legend><div class="controls">
      {#each [1,2,3,4,5] as value}<label>{t('engineering-grade', { grade: integer(value) })}<input data-testid={`rolls-${value}`} type="number" min="1" max="10" step="1" bind:value={rolls[value-1]} oninput={edit} /></label>{/each}
    </div></fieldset>
    <div class="controls"><button data-testid="preview" disabled={!valid || busy || awaitingPreview || engineeringActions.pending || selectionPending} onclick={() => send('preview', { blueprint, experimental: experimental || null, grade, rolls: [...rolls] })}>{t('engineering-preview')}</button>
    <button data-testid="close-flow" disabled={busy || engineeringActions.pending || selectionPending} onclick={() => send('close_flow', {})}>{t('engineering-close-flow')}</button></div>
    {#if data?.status === 'no_data'}<p>{t('engineering-no-data')}</p>{/if}
    {#if !calculated}<p>{t('engineering-recalculate')}</p>{/if}
    {#if data?.cost && calculated}
      {#if data.cost.replaces_blueprint}<p class="warning">{t('engineering-replaces-blueprint')}</p>{/if}
      {#if !data.cost.verified}<p>{t('engineering-unverified')}</p>{/if}
      {#if data.cost.missing_grades.length}<p>{t('engineering-no-data')}: {data.cost.missing_grades.map(g => t('engineering-grade', { grade: integer(g) })).join(', ')}</p>{/if}
      <h3>{t('engineering-total-materials')}</h3><p>{t('engineering-ready-percent', { pct: integer(data.percent) })}</p>
      <div class="scroll"><table><thead><tr><th>{t('engineering-col-material')}</th><th>{t('engineering-col-need')}</th><th>{t('engineering-col-have')}</th><th>{t('engineering-col-status')}</th></tr></thead><tbody>
        {#each data.lines as row (row.key)}<tr data-material={row.key}><td>{name(row.key)}</td><td>{integer(row.need)}</td><td>{integer(row.have)}</td><td>{t(`engineering-status-${row.state}`, { missing: integer(row.missing) })}
        {#if row.trade}<p>{t('engineering-trade-proposal', { give: integer(row.trade.give), material: name(row.trade.give_key), receive: `${integer(row.trade.get)} ${name(row.trade.get_key)}` })}</p>{/if}</td></tr>{/each}
      </tbody></table></div>
      <div class="costs">
        {#each Object.entries(data.cost.per_grade) as [level, [count, ingredients]]}<section data-testid={`cost-grade-${level}`}><h4>{t('engineering-grade', { grade: level })} · {t('engineering-rolls')}: {integer(count)}</h4><ul>{#each Object.entries(ingredients) as [key, count]}<li>{name(key)}: {integer(count)}</li>{/each}</ul></section>{/each}
        <section data-testid="cost-experimental"><h4>{t('engineering-col-experimental')}</h4><ul>{#each Object.entries(data.cost.experimental ?? {}) as [key, count]}<li>{name(key)}: {integer(count)}</li>{/each}</ul></section>
      </div>
      <section data-testid="flow-engineer"><h3>{t('engineering-col-engineer')}</h3>
        {#if recommended}<p>{recommended.name} · {engineerStatusLabel(recommended.status)} · {recommended.system ?? t('engineering-no-data')} · {recommended.distance_ly == null ? t('engineering-no-data') : t('engineering-distance-ly', { distance: decimal(recommended.distance_ly) })}</p>
        {#if !recommended.verified}<p>{t('engineering-unverified')}</p>{/if}
        {#if !data.engineer?.nearest_unlocked}<h4>{t('engineering-engineer-next-step')}</h4><p data-testid="flow-next-step">
          {#if next && typeof next === 'object' && 'Requirement' in next}{requirement(next.Requirement.key)}
          {:else if next && typeof next === 'object' && 'Invite' in next}{requirement(next.Invite.key)}
          {:else if next && typeof next === 'object' && 'Contribute' in next}
            {@const contribution = next.Contribute}
            {@const item = contribution.kind === 'credits' ? { name: t('engineering-contribution-credits'), pending: false } : contribution.kind === 'commodity' ? contributionCommodityLabel(contribution.item) : { name: name(contribution.item), pending: false }}
            {item.name}: {t('engineering-engineer-progress', { done: integer(contribution.have), need: integer(contribution.need) })}{#if item.pending} · {t('engineering-translation-pending')}{/if}
          {:else}{t('engineering-no-data')}{/if}
        </p>{/if}{:else}<p>{t('engineering-no-data')}</p>{/if}
      </section>
      <button data-testid="pin-goal" disabled={busy || awaitingPreview || engineeringActions.pending || selectionPending || data.status !== 'ok'} onclick={() => send('pin_goal', {})}>{t('engineering-add-to-shopping')}</button>
    {/if}
  {/if}
</Panel>
<style>
  .controls, .costs { display: flex; flex-wrap: wrap; gap: .6rem; margin: .6rem 0; }
  label { display: flex; flex-direction: column; gap: .3rem; }
  input { width: 4rem; }
  button, select, input { background: var(--hud-bg); color: var(--hud-primary); border: 1px solid var(--hud-border); padding: .35rem; }
  button:disabled { opacity: .5; }
  fieldset { border: 1px solid var(--hud-border); }
  .scroll { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: .4rem; text-align: left; border-bottom: 1px solid var(--hud-border); }
  td p { margin: .3rem 0; }
  section { overflow-wrap: anywhere; }
  h3, h4 { color: var(--hud-primary); margin: .6rem 0; }
  ul { padding-left: 1.2rem; }
  .warning { color: var(--hud-warning); }
  [role=alert] { color: var(--hud-danger); }
</style>

