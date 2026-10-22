<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { integer, decimal } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import { engineeringActions, shipFlowAction, syncEngineeringContext } from './actions.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blockerLabel, blueprintLabel, engineeringDataLabel, engineerStatusLabel, materialName, contributionCommodityLabel } from './labels';
  import { pushToast } from '../../shell/Toasts.svelte';
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
  async function copyText(text: string) {
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
        pushToast(`${t('engineering-copied')}: ${text}`, 'Info');
      }
    } catch { /* Ignorado en entornos sin portapapeles */ }
  }
</script>

<Panel titleKey="panel-engineering-flow" span={2}>
  {#if !slot}
    <p class="muted">{t('engineering-ship-pick-module')}</p>
  {:else if !rest.ready}
    <p role="status">{t(rest.state === 'failed' ? 'engineering-page-failed' : rest.state === 'stale' ? 'engineering-data-stale' : 'engineering-page-loading')}</p>
  {:else}
    <div class="flow-header">
      <div class="module-title">
        <span class="mod-name">{data?.module_type ? engineeringDataLabel('modtype', data.module_type).name : data?.item}</span>
        <span class="mod-slot">· {slot}</span>
      </div>
    </div>

    {#if failed}<p role="alert" class="error-msg">{t('engineering-action-failed')}</p>{/if}

    <div class="controls-card">
      <div class="controls">
        <label>
          {t('engineering-col-blueprint')}
          <select data-testid="blueprint" bind:value={blueprint} onchange={edit}>
            {#each blueprintRows as option (option.key)}
              {@const label = blueprintLabel(option.key)}
              <option value={option.key} disabled={!option.available}>
                {option.available ? label.name : t('engineering-blueprint-unavailable', { name: label.name, reason: blockerLabel(option.reason ?? '') })}
                {label.pending ? ` · ${t('engineering-translation-pending')}` : ''}
              </option>
            {/each}
          </select>
        </label>

        <label>
          {t('engineering-col-experimental')}
          <select data-testid="experimental" bind:value={experimental} onchange={edit}>
            <option value="">{t('engineering-no-experimental')}</option>
            {#each experimentalRows as option (option.key)}
              {@const label = engineeringDataLabel('exp', option.key)}
              <option value={option.key}>
                {label.name}
                {label.pending ? ` · ${t('engineering-translation-pending')}` : ''}
              </option>
            {/each}
          </select>
        </label>

        <label>
          {t('engineering-target-grade')}
          <select data-testid="target-grade" bind:value={grade} onchange={edit}>
            {#each applicable as value (value)}
              <option {value}>{t('engineering-grade', { grade: integer(value) })}</option>
            {/each}
          </select>
        </label>
      </div>

      <fieldset class="rolls-fieldset">
        <legend>{t('engineering-rolls')}</legend>
        <div class="rolls-grid">
          {#each rollGrades as value (value)}
            <label class="roll-item">
              <span class="roll-label">{t('engineering-grade', { grade: integer(value) })}</span>
              <input data-testid={`rolls-${value}`} type="number" min="1" max="10" step="1" bind:value={rolls[value-1]} oninput={edit} />
            </label>
          {/each}
        </div>
      </fieldset>

      {#if selectedOption && !selectedOption.available}
        <p class="warning" data-testid="blueprint-blocked">{blockerLabel(selectedOption.reason ?? '')}</p>
      {/if}

      <div class="action-buttons">
        <button
          data-testid="preview"
          class="btn-primary"
          disabled={!valid || busy || awaitingPreview || engineeringActions.pending || selectionPending}
          onclick={() => send('preview', { blueprint, experimental: experimental || null, grade, rolls: [...rolls] })}
        >
          {t('engineering-preview')}
        </button>
        <button
          data-testid="close-flow"
          class="btn-secondary"
          disabled={busy || engineeringActions.pending || selectionPending}
          onclick={() => send('close_flow', {})}
        >
          {t('engineering-close-flow')}
        </button>
      </div>
    </div>

    {#if !calculated}
      <p class="recalc-hint">{t('engineering-recalculate')}</p>
    {/if}

    {#if calculated && data?.status === 'no_data' && !data.blockers.length}
      <p class="muted">{t('engineering-no-data')}</p>
    {/if}

    {#if calculated && data?.blockers.length}
      <section data-testid="flow-blockers" class="blockers-card">
        <h3>{t('engineering-calculation-blocked')}</h3>
        <ul>
          {#each data.blockers as blocker}
            <li>{blockerLabel(blocker)}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data && calculated && data.status === 'ok'}
      {#if data.replaces_blueprint}
        <p class="warning">{t('engineering-replaces-blueprint')}</p>
      {/if}
      {#if !data.verified}
        <p class="muted">{t('engineering-unverified')}</p>
      {/if}

      <!-- Recommended Engineer Card at Top -->
      <section data-testid="flow-engineer" class="engineer-card-highlight">
        <div class="eng-card-header">
          <span class="eng-card-badge">{t('engineering-recommended-engineer')}</span>
          {#if recommended}
            <span class="eng-status-pill">{engineerStatusLabel(recommended.status)}</span>
          {/if}
        </div>
        {#if recommended}
          <div class="eng-card-body">
            <h3 class="eng-card-name">{recommended.name}</h3>
            <div class="eng-card-info">
              <span>{recommended.system ?? t('engineering-no-data')}</span>
              {#if recommended.system}
                <button
                  type="button"
                  class="btn-copy"
                  title={t('engineering-copy-system')}
                  onclick={() => copyText(recommended.system!)}
                >
                  📋
                </button>
              {/if}
              <span>·</span>
              <span class="mono">{recommended.distance_ly == null ? t('engineering-no-data') : t('engineering-distance-ly', { distance: decimal(recommended.distance_ly) })}</span>
            </div>
          </div>
          {#if !recommended.verified}
            <small class="muted">{t('engineering-unverified')}</small>
          {/if}
          {#if !data.engineer?.nearest_unlocked}
            <div class="eng-next-step-box">
              <h4>{t('engineering-engineer-next-step')}</h4>
              <p data-testid="flow-next-step">
                {#if next && typeof next === 'object' && 'Requirement' in next}
                  {requirement(next.Requirement.key)}
                {:else if next && typeof next === 'object' && 'Invite' in next}
                  {requirement(next.Invite.key)}
                {:else if next && typeof next === 'object' && 'Contribute' in next}
                  {@const contribution = next.Contribute}
                  {@const item = contribution.kind === 'credits' ? { name: t('engineering-contribution-credits'), pending: false } : contribution.kind === 'commodity' ? contributionCommodityLabel(contribution.item) : { name: name(contribution.item), pending: false }}
                  {item.name}: {t('engineering-engineer-progress', { done: integer(contribution.have), need: integer(contribution.need) })}{#if item.pending} · {t('engineering-translation-pending')}{/if}
                {:else}
                  {t('engineering-no-data')}
                {/if}
              </p>
            </div>
          {/if}
        {:else}
          <p class="muted">{t('engineering-no-engineer-covers')}</p>
        {/if}
      </section>

      <!-- Stepper por Grados (G1 a G5) -->
      <div class="stepper-section">
        <h3 class="stepper-title">{t('engineering-rolls')}</h3>
        <div class="costs stepper-grid">
          {#each gradeRows as row (row.grade)}
            <section data-testid={`cost-grade-${row.grade}`} class="grade-card" class:target-grade={row.grade === data.request?.target_grade}>
              <div class="grade-card-header">
                <span class="grade-badge">G{row.grade}</span>
                <span class="grade-rolls-badge">{t('engineering-step-rolls-count', { rolls: integer(row.rolls) })}</span>
              </div>
              <h4>{t('engineering-grade', { grade: integer(row.grade) })} · {t('engineering-rolls')}: {integer(row.rolls)}</h4>
              <ul class="cost-list">
                {#each costRows.filter(cost => cost.grade === row.grade && cost.kind === 'material') as cost (cost.material)}
                  <li>
                    <span class="cost-name">{name(cost.material ?? '')}:</span>
                    <span class="cost-count font-mono">{integer(cost.count)}</span>
                  </li>
                {/each}
                {#if row.merc_coins > 0}
                  <li class="merc-cost">{t('engineering-merc-amount', { count: integer(row.merc_coins) })}</li>
                {/if}
              </ul>
            </section>
          {/each}

          <section data-testid="cost-experimental" class="grade-card exp-card">
            <div class="grade-card-header">
              <span class="grade-badge exp-badge">EXP</span>
            </div>
            <h4>{t('engineering-col-experimental')}</h4>
            <ul class="cost-list">
              {#each costRows.filter(cost => cost.grade === null && cost.kind === 'material') as cost (cost.material)}
                <li>
                  <span class="cost-name">{name(cost.material ?? '')}:</span>
                  <span class="cost-count font-mono">{integer(cost.count)}</span>
                </li>
              {/each}
              {#each costRows.filter(cost => cost.grade === null && cost.kind === 'merc_coins') as cost}
                <li class="merc-cost">{t('engineering-merc-amount', { count: integer(cost.count) })}</li>
              {/each}
            </ul>
          </section>
        </div>
      </div>

      <!-- Materials Total & Semaphore Table -->
      <div class="materials-section">
        <div class="mat-header-bar">
          <h3>{t('engineering-total-materials')}</h3>
          <p data-testid="materials-percent" class="mat-percent-badge">{t('engineering-ready-percent', { pct: integer(data.percent) })}</p>
        </div>

        <MercCoinsLine coins={data.merc_coins} />
        {#if data.materials_ready && data.merc_coins && data.merc_coins.need > 0 && data.merc_coins.have === null}
          <p class="warning" data-testid="merc-not-confirmed">{t('engineering-merc-not-confirmed')}</p>
        {/if}

        <div class="scroll">
          <table class="mat-table">
            <thead>
              <tr>
                <th>{t('engineering-col-material')}</th>
                <th>{t('engineering-col-need')}</th>
                <th>{t('engineering-col-have')}</th>
                <th>{t('engineering-col-status')}</th>
              </tr>
            </thead>
            <tbody>
              {#each lineRows as row (row.key)}
                <tr data-material={row.key} class="mat-row state-{row.state}">
                  <td class="mat-name-cell">
                    <strong>{name(row.key)}</strong>
                  </td>
                  <td class="mono">{integer(row.need)}</td>
                  <td class="mono">{integer(row.have)}</td>
                  <td>
                    <span class="status-indicator status-{row.state}">
                      {t(`engineering-status-${row.state}`, { missing: integer(row.missing) })}
                    </span>
                    {#if row.trade}
                      <div class="trade-box">
                        <span class="trade-icon">⇄</span>
                        <p>{t('engineering-trade-proposal', { give: integer(row.trade.give), material: name(row.trade.give_key), receive: `${integer(row.trade.get)} ${name(row.trade.get_key)}` })}</p>
                      </div>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <div class="pin-goal-wrap">
        <button
          data-testid="pin-goal"
          class="btn-pin"
          disabled={busy || awaitingPreview || engineeringActions.pending || selectionPending || data.status !== 'ok'}
          onclick={() => send('pin_goal', {})}
        >
          {t('engineering-add-to-shopping')}
        </button>
      </div>
    {/if}
  {/if}
</Panel>

<style>
  .flow-header { margin-bottom: .6rem; }
  .mod-name { font-size: 1rem; font-weight: 700; color: var(--hud-primary); }
  .mod-slot { font-size: .85rem; color: var(--hud-muted); }

  .controls-card { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--hud-border); border-radius: 6px; padding: .8rem; margin-bottom: .8rem; }
  .controls { display: flex; flex-wrap: wrap; gap: .8rem; margin-bottom: .6rem; }
  label { display: flex; flex-direction: column; gap: .3rem; font-size: .82rem; }
  select, input { background: var(--hud-bg); color: var(--hud-text); border: 1px solid var(--hud-muted); padding: .35rem .5rem; border-radius: 4px; font-size: .85rem; }
  select:focus, input:focus { border-color: var(--hud-primary); outline: none; }

  .rolls-fieldset { border: 1px solid var(--hud-border); border-radius: 4px; padding: .5rem .8rem; margin-bottom: .6rem; }
  .rolls-grid { display: flex; flex-wrap: wrap; gap: .6rem; }
  .roll-item { flex-direction: row; align-items: center; gap: .4rem; }
  .roll-item input { width: 3.5rem; text-align: center; }

  .action-buttons { display: flex; gap: .6rem; margin-top: .6rem; }
  .btn-primary { background: rgba(255, 113, 0, 0.2); color: var(--hud-primary); border: 1px solid var(--hud-primary); padding: .4rem .8rem; border-radius: 4px; font-weight: 600; cursor: pointer; }
  .btn-primary:hover:not(:disabled) { background: var(--hud-primary); color: #000; }
  .btn-secondary { background: var(--hud-bg); color: var(--hud-muted); border: 1px solid var(--hud-border); padding: .4rem .8rem; border-radius: 4px; cursor: pointer; }
  .btn-secondary:hover:not(:disabled) { color: var(--hud-text); border-color: var(--hud-text); }
  button:disabled { opacity: .4; cursor: not-allowed; }

  .engineer-card-highlight { background: rgba(255, 113, 0, 0.06); border: 1px solid rgba(255, 113, 0, 0.35); border-radius: 6px; padding: .8rem; margin: .8rem 0; }
  .eng-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: .4rem; }
  .eng-card-badge { font-size: .7rem; font-weight: 700; text-transform: uppercase; color: var(--hud-primary); letter-spacing: .5px; }
  .eng-status-pill { font-size: .7rem; font-weight: 600; padding: .1rem .4rem; border-radius: 3px; background: rgba(0, 255, 100, 0.15); color: #00ff66; border: 1px solid rgba(0, 255, 100, 0.3); }
  .eng-card-name { font-size: 1.05rem; font-weight: 700; color: #fff; margin: 0 0 .2rem; }
  .eng-card-info { font-size: .82rem; color: var(--hud-muted); display: flex; gap: .4rem; align-items: center; }

  .eng-next-step-box { background: rgba(0, 0, 0, 0.25); border: 1px solid var(--hud-border); border-radius: 4px; padding: .5rem; margin-top: .6rem; }
  .eng-next-step-box h4 { font-size: .8rem; margin: 0 0 .2rem; color: var(--hud-text); }
  .eng-next-step-box p { font-size: .8rem; margin: 0; color: var(--hud-text); }

  .stepper-section { margin: 1rem 0; }
  .stepper-title { font-size: .92rem; color: var(--hud-primary); margin: 0 0 .5rem; }
  .stepper-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(130px, 1fr)); gap: .5rem; }
  .grade-card { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--hud-border); border-radius: 5px; padding: .6rem; display: flex; flex-direction: column; gap: .3rem; }
  .grade-card.target-grade { border-color: var(--hud-primary); background: rgba(255, 113, 0, 0.06); }
  .grade-card h4 { font-size: .78rem; margin: 0; color: var(--hud-text); }
  .grade-card-header { display: flex; justify-content: space-between; align-items: center; }
  .grade-badge { font-size: .75rem; font-weight: 800; color: var(--hud-primary); }
  .grade-rolls-badge { font-size: .68rem; color: var(--hud-muted); font-weight: 600; }
  .exp-card { border-style: dashed; }
  .exp-badge { color: #00d2ff; }

  .cost-list { list-style: none; padding: 0; margin: 0; font-size: .75rem; }
  .cost-list li { display: flex; justify-content: space-between; padding: .1rem 0; }
  .cost-name { color: var(--hud-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cost-count { font-weight: 600; }
  .merc-cost { color: #ffcc00; font-weight: 600; }
  .btn-copy { background: transparent; border: 1px solid var(--hud-border); border-radius: 3px; color: var(--hud-muted); padding: .1rem .3rem; font-size: .75rem; cursor: pointer; line-height: 1; }
  .btn-copy:hover { color: var(--hud-primary); border-color: var(--hud-primary); }

  .materials-section { margin: 1rem 0; }
  .mat-header-bar { display: flex; justify-content: space-between; align-items: center; margin-bottom: .5rem; }
  .mat-header-bar h3 { font-size: .95rem; color: var(--hud-primary); margin: 0; }
  .mat-percent-badge { font-size: .85rem; font-weight: 700; color: #00ff66; margin: 0; }

  .scroll { overflow-x: auto; }
  .mat-table { width: 100%; border-collapse: collapse; font-size: .85rem; }
  .mat-table th, .mat-table td { padding: .5rem; text-align: left; border-bottom: 1px solid var(--hud-border); }
  .mat-table th { background: rgba(0, 0, 0, 0.2); font-size: .75rem; text-transform: uppercase; color: var(--hud-muted); }

  .status-indicator { font-size: .78rem; font-weight: 600; padding: .15rem .4rem; border-radius: 3px; display: inline-block; }
  .status-ok { color: #00ff66; background: rgba(0, 255, 100, 0.1); }
  .status-trade { color: #00d2ff; background: rgba(0, 210, 255, 0.1); }
  .status-missing { color: #ff5555; background: rgba(255, 85, 85, 0.1); }

  .trade-box { background: rgba(0, 210, 255, 0.05); border: 1px solid rgba(0, 210, 255, 0.25); border-radius: 4px; padding: .3rem .5rem; margin-top: .3rem; font-size: .78rem; display: flex; align-items: center; gap: .4rem; }
  .trade-box p { margin: 0; color: var(--hud-text); }
  .trade-icon { color: #00d2ff; font-weight: bold; }

  .pin-goal-wrap { margin-top: .8rem; }
  .btn-pin { width: 100%; background: rgba(0, 255, 100, 0.15); color: #00ff66; border: 1px solid rgba(0, 255, 100, 0.4); padding: .5rem; border-radius: 4px; font-weight: 700; cursor: pointer; }
  .btn-pin:hover:not(:disabled) { background: #00ff66; color: #000; }

  .mono { font-family: var(--font-mono); }
  .muted { color: var(--hud-muted); }
  .warning { color: var(--hud-warning); font-size: .85rem; }
  .error-msg { color: var(--hud-danger); font-size: .85rem; }
</style>
