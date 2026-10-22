<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { integer, decimal } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import { engineeringActions, shipFlowAction, syncEngineeringContext } from './actions.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blockerLabel, blueprintLabel, engineeringDataLabel, engineerStatusLabel, materialName, contributionCommodityLabel } from './labels';
  import { createSectionsReader } from './sections.svelte';
  import MercCoinsLine from './MercCoins.svelte';
  import type { EngineeringFlowTopic, FlowRows, InventoryTopic } from './types';

  const data = $derived(topic<EngineeringFlowTopic>('engineering.flow'));
  // La visibilidad del flujo la decide la ranura abierta, no el estado del cálculo.
  const slot = $derived(data?.slot ?? null);
  const commander = $derived(topic<{ name?: string }>('core.cmdr')?.name ?? '');
  // Opciones, grados, costes y líneas llegan por REST paginado, completos y de esta revisión y ranura.
  const rest = createSectionsReader<FlowRows>('flow', ['blueprints', 'experimentals', 'grades', 'costs', 'lines'],
    () => ({ revision: data?.revision, commander, enabled: slot !== null, scope: slot ?? undefined }));
  const blueprintRows = $derived(rest.ready ? rest.blueprints.items : []);
  const experimentalRows = $derived(rest.ready ? rest.experimentals.items : []);
  const lineRows = $derived(rest.ready ? rest.lines.items : []);
  const gradeRows = $derived(rest.ready ? rest.grades.items : []);
  const costRows = $derived(rest.ready ? rest.costs.items : []);
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
  const selectedOption = $derived(blueprintRows.find(row => row.key === blueprint));
  // Solo grados que la receta resuelta tiene de verdad (p. ej. sin G1 para una base comprada).
  const applicable = $derived(selectedOption?.available ? [...selectedOption.grades].sort((a, b) => a - b) : []);
  const rollGrades = $derived(applicable.filter(value => value <= grade));
  const draft = $derived(JSON.stringify({ blueprint, experimental: experimental || null, grade, rolls: [...rolls] }));
  const confirmed = $derived(data?.request ? JSON.stringify({ blueprint: data.request.blueprint, experimental: data.request.experimental, grade: data.request.target_grade, rolls: data.request.rolls }) : null);
  const awaitingPreview = $derived(pendingPreview !== null && pendingPreview !== confirmed);
  const calculated = $derived(confirmed === draft);
  const selectionPending = $derived(engineeringActions.selectedSlot !== null && engineeringActions.selectedSlot !== slot);
  const valid = $derived(blueprint !== '' && selectedOption?.available === true && applicable.includes(grade) && rolls.length === 5 && rolls.every(n => Number.isInteger(n) && n >= 1 && n <= 10));

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
    blueprint = data?.request?.blueprint ?? '';
    experimental = data?.request?.experimental ?? '';
    grade = data?.request?.target_grade ?? 5;
    rolls = [...(data?.request?.rolls ?? defaults)];
  });

  // Con las opciones completas: nunca queda elegida una receta no disponible ni un grado inaplicable.
  $effect(() => {
    if (!rest.ready) return;
    const available = blueprintRows.filter(row => row.available);
    if (!available.some(row => row.key === blueprint)) blueprint = available[0]?.key ?? blueprintRows[0]?.key ?? '';
    const grades = blueprintRows.find(row => row.key === blueprint)?.grades ?? [];
    if (grades.length && !grades.includes(grade)) grade = grades.includes(5) ? 5 : Math.max(...grades);
    if (experimental && !experimentalRows.some(row => row.key === experimental)) experimental = '';
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
    return lineRows.find(row => row.key === key)?.name_en ?? key;
  }
  function requirement(key: string): string { const text = t(key); return text === key ? t('engineering-no-data') : text; }
</script>
<Panel titleKey="panel-engineering-flow" span={2}>
  {#if !slot}<p>{t('engineering-ship-pick-module')}</p>
  {:else if !rest.ready}
    <p role="status">{t(rest.state === 'failed' ? 'engineering-page-failed' : rest.state === 'stale' ? 'engineering-flow-stale' : 'engineering-page-loading')}</p>
  {:else}
    <p>{data?.module_type ? engineeringDataLabel('modtype', data.module_type).name : data?.item} · {slot}</p>
    {#if failed}<p role="alert">{t('engineering-action-failed')}</p>{/if}
    <div class="controls">
      <label>{t('engineering-col-blueprint')}<select data-testid="blueprint" bind:value={blueprint} onchange={edit}>
        {#each blueprintRows as option (option.key)}{@const label = blueprintLabel(option.key)}<option value={option.key} disabled={!option.available}>{option.available ? label.name : t('engineering-blueprint-unavailable', { name: label.name, reason: blockerLabel(option.reason ?? '') })}{label.pending ? ` · ${t('engineering-translation-pending')}` : ''}</option>{/each}
      </select></label>
      <label>{t('engineering-col-experimental')}<select data-testid="experimental" bind:value={experimental} onchange={edit}>
        <option value="">{t('engineering-no-experimental')}</option>
        {#each experimentalRows as option (option.key)}{@const label = engineeringDataLabel('exp', option.key)}<option value={option.key}>{label.name}{label.pending ? ` · ${t('engineering-translation-pending')}` : ''}</option>{/each}
      </select></label>
      <label>{t('engineering-target-grade')}<select data-testid="target-grade" bind:value={grade} onchange={edit}>{#each applicable as value (value)}<option {value}>{t('engineering-grade', { grade: integer(value) })}</option>{/each}</select></label>
    </div>
    <fieldset><legend>{t('engineering-rolls')}</legend><div class="controls">
      {#each rollGrades as value (value)}<label>{t('engineering-grade', { grade: integer(value) })}<input data-testid={`rolls-${value}`} type="number" min="1" max="10" step="1" bind:value={rolls[value-1]} oninput={edit} /></label>{/each}
    </div></fieldset>
    {#if selectedOption && !selectedOption.available}<p class="warning" data-testid="blueprint-blocked">{blockerLabel(selectedOption.reason ?? '')}</p>{/if}
    <div class="controls"><button data-testid="preview" disabled={!valid || busy || awaitingPreview || engineeringActions.pending || selectionPending} onclick={() => send('preview', { blueprint, experimental: experimental || null, grade, rolls: [...rolls] })}>{t('engineering-preview')}</button>
    <button data-testid="close-flow" disabled={busy || engineeringActions.pending || selectionPending} onclick={() => send('close_flow', {})}>{t('engineering-close-flow')}</button></div>
    {#if !calculated}<p>{t('engineering-recalculate')}</p>{/if}
    {#if calculated && data?.status === 'no_data' && !data.blockers.length}<p>{t('engineering-no-data')}</p>{/if}
    {#if calculated && data?.blockers.length}
      <section data-testid="flow-blockers"><h3>{t('engineering-calculation-blocked')}</h3><ul>{#each data.blockers as blocker}<li>{blockerLabel(blocker)}</li>{/each}</ul></section>
    {/if}
    {#if data && calculated && data.status === 'ok'}
      {#if data.replaces_blueprint}<p class="warning">{t('engineering-replaces-blueprint')}</p>{/if}
      {#if !data.verified}<p>{t('engineering-unverified')}</p>{/if}
      <h3>{t('engineering-total-materials')}</h3><p data-testid="materials-percent">{t('engineering-ready-percent', { pct: integer(data.percent) })}</p>
      <MercCoinsLine coins={data.merc_coins} />
      {#if data.materials_ready && data.merc_coins && data.merc_coins.need > 0 && data.merc_coins.have === null}<p class="warning" data-testid="merc-not-confirmed">{t('engineering-merc-not-confirmed')}</p>{/if}
      <div class="scroll"><table><thead><tr><th>{t('engineering-col-material')}</th><th>{t('engineering-col-need')}</th><th>{t('engineering-col-have')}</th><th>{t('engineering-col-status')}</th></tr></thead><tbody>
        {#each lineRows as row (row.key)}<tr data-material={row.key}><td>{name(row.key)}</td><td>{integer(row.need)}</td><td>{integer(row.have)}</td><td>{t(`engineering-status-${row.state}`, { missing: integer(row.missing) })}
        {#if row.trade}<p>{t('engineering-trade-proposal', { give: integer(row.trade.give), material: name(row.trade.give_key), receive: `${integer(row.trade.get)} ${name(row.trade.get_key)}` })}</p>{/if}</td></tr>{/each}
      </tbody></table></div>
      <div class="costs">
        {#each gradeRows as row (row.grade)}<section data-testid={`cost-grade-${row.grade}`}><h4>{t('engineering-grade', { grade: integer(row.grade) })} · {t('engineering-rolls')}: {integer(row.rolls)}</h4><ul>{#each costRows.filter(cost => cost.grade === row.grade && cost.kind === 'material') as cost (cost.material)}<li>{name(cost.material ?? '')}: {integer(cost.count)}</li>{/each}{#if row.merc_coins > 0}<li>{t('engineering-merc-amount', { count: integer(row.merc_coins) })}</li>{/if}</ul></section>{/each}
        <section data-testid="cost-experimental"><h4>{t('engineering-col-experimental')}</h4><ul>{#each costRows.filter(cost => cost.grade === null && cost.kind === 'material') as cost (cost.material)}<li>{name(cost.material ?? '')}: {integer(cost.count)}</li>{/each}{#each costRows.filter(cost => cost.grade === null && cost.kind === 'merc_coins') as cost}<li>{t('engineering-merc-amount', { count: integer(cost.count) })}</li>{/each}</ul></section>
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
        </p>{/if}{:else}<p>{t('engineering-no-engineer-covers')}</p>{/if}
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
