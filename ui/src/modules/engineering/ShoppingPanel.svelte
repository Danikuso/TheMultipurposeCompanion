<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer, decimal } from '../../lib/format';
  import { action, live, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { createGoalsReader } from './goals.svelte';
  import { blockerLabel, blueprintLabel, engineeringDataLabel, inventoryMaterial } from './labels';
  import MercCoinsLine from './MercCoins.svelte';
  import type { GoalsTopic, InventoryTopic } from './types';

  const reader = createGoalsReader(true);
  const inventory = $derived(topic<InventoryTopic>('engineering.inventory'));
  const summary = $derived(topic<GoalsTopic>('engineering.goals'));
  let pending = $state(false);
  let error = $state(false);
  const commander = $derived(topic<{ name?: string }>('core.cmdr')?.name ?? '');
  let generation = 0;

  $effect(() => {
    commander;
    generation++;
    error = false;
    return () => { generation++; };
  });

  async function update(name: string, args: unknown) {
    if (pending) return;
    pending = true; error = false; const current = generation;
    try { await action(`engineering.${name}`, args); }
    catch { if (current === generation) error = true; }
    finally { pending = false; }
  }
</script>

<Panel titleKey="panel-engineering-shopping" span={2} collapsible={true} defaultExpanded={false}>
  {#if error}<p role="alert" class="error-msg">{t('engineering-action-failed')}</p>{/if}

  {#if reader.ready}
    {#if !reader.goals.items.length}
      <p class="muted">{t('engineering-no-data')}</p>
    {/if}

    <div class="goals-container">
      {#each reader.goals.items as goal, index (goal.id)}
        {@const blueprint = blueprintLabel(goal.request.blueprint)}
        {@const module = engineeringDataLabel('modtype', goal.request.module_type)}
        {@const experimental = goal.request.experimental ? engineeringDataLabel('exp', goal.request.experimental) : null}
        <article data-goal-id={goal.id} class="goal-card" class:goal-completable={goal.completable}>
          <div class="goal-card-header">
            <div class="goal-title-wrap">
              <h3 class="goal-title">
                {module.name} · {goal.slot} · {blueprint.name} {t('engineering-grade', { grade: integer(goal.request.target_grade) })}
              </h3>
              {#if module.pending}<small class="pending-tag">{t('engineering-translation-pending')}</small>{/if}
              {#if blueprint.pending}<small class="pending-tag">{t('engineering-translation-pending')}</small>{/if}
            </div>
            <div class="goal-badge-wrap">
              <strong class="percent-badge" class:ready-badge={goal.percent >= 100}>
                {t('engineering-ready-percent', { pct: decimal(goal.percent) })}
              </strong>
              {#if !goal.verified}<small class="unverified-tag">{t('engineering-unverified')}</small>{/if}
            </div>
          </div>

          <div class="goal-subinfo">
            <p class="exp-text">
              {experimental?.name ?? t('engineering-no-experimental')}
              {#if experimental?.pending}<small class="pending-tag">{t('engineering-translation-pending')}</small>{/if}
            </p>
            <p class="rolls-text font-mono">
              {t('engineering-rolls')}: {goal.request.rolls.map(integer).join(' / ')}
            </p>
          </div>

          <MercCoinsLine coins={goal.merc_coins} />

          {#if goal.blockers.length}
            <ul data-testid="goal-blockers" class="blockers-list">
              {#each goal.blockers as blocker}
                <li>{blockerLabel(blocker)}</li>
              {/each}
            </ul>
          {/if}

          {#if goal.materials_ready && !goal.completable && !goal.blockers.length && goal.merc_coins.need > 0 && goal.merc_coins.have === null}
            <p data-testid="goal-merc-not-confirmed" class="warning">{t('engineering-merc-not-confirmed')}</p>
          {/if}

          {#if goal.completable && goal.engineer}
            <div class="completable-box">
              <p class="ready">{t('engineering-goal-completable', { engineer: goal.engineer })}</p>
            </div>
          {/if}

          {#if Object.keys(goal.missing).length > 0}
            <div class="missing-section">
              <ul class="missing-list">
                {#each Object.entries(goal.missing) as [key, count]}
                  <li>
                    <span class="missing-name">{inventoryMaterial(key, inventory)}:</span>
                    <span class="missing-count">{t('engineering-goal-missing')}: {integer(count)}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}

          <div class="actions">
            <button
              data-testid={`goal-up-${index}`}
              class="btn-act"
              disabled={pending || !live.connected || index === 0}
              onclick={() => update('move_goal', { id: goal.id, to: index - 1 })}
            >
              ▲ {t('engineering-goal-up')}
            </button>
            <button
              data-testid={`goal-down-${index}`}
              class="btn-act"
              disabled={pending || !live.connected || index >= (summary?.total ?? 0) - 1}
              onclick={() => update('move_goal', { id: goal.id, to: index + 1 })}
            >
              ▼ {t('engineering-goal-down')}
            </button>
            <button
              data-testid={`goal-remove-${index}`}
              class="btn-act btn-remove"
              disabled={pending || !live.connected}
              onclick={() => update('unpin_goal', { id: goal.id })}
            >
              ✕ {t('engineering-goal-remove')}
            </button>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <p role="status">
      {t(reader.state === 'failed' ? 'engineering-page-failed' : reader.state === 'stale' ? 'engineering-goals-stale' : reader.state === 'idle' ? 'engineering-no-data' : 'engineering-page-loading')}
    </p>
  {/if}

  {#snippet details()}
    {#if reader.ready}
      <!-- Resumen acumulado (Needs / Missing / Surplus) en tarjetas -->
      <div class="aggregates-grid">
        {#each ['needs', 'missing', 'surplus'] as section}
          <section data-section={section} class="aggregate-card aggregate-{section}">
            <h3 class="aggregate-title">{t(`engineering-aggregate-${section}`)}</h3>
            <ul class="aggregate-list">
              {#each reader[section as 'needs' | 'missing' | 'surplus'].items as row (row.key)}
                <li class="aggregate-item">
                  <span class="agg-name">{inventoryMaterial(row.key, inventory, row.name_en)}:</span>
                  <span class="agg-count font-mono">{integer(row.count)}</span>
                </li>
              {:else}
                <li class="muted">{t('engineering-no-data')}</li>
              {/each}
            </ul>
          </section>
        {/each}
      </div>
    {/if}
  {/snippet}
</Panel>

<style>
  .goals-container { display: flex; flex-direction: column; gap: .75rem; margin-bottom: 1rem; }
  .goal-card { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--hud-border); border-radius: 6px; padding: .8rem; overflow-wrap: anywhere; }
  .goal-card.goal-completable { border-color: rgba(0, 255, 100, 0.5); background: rgba(0, 255, 100, 0.04); }

  .goal-card-header { display: flex; justify-content: space-between; align-items: flex-start; gap: .5rem; margin-bottom: .4rem; }
  .goal-title { font-size: .95rem; font-weight: 700; color: var(--hud-primary); margin: 0; }
  .goal-badge-wrap { display: flex; align-items: center; gap: .5rem; white-space: nowrap; }

  .percent-badge { font-size: .85rem; font-weight: 700; color: var(--hud-primary); background: rgba(255, 113, 0, 0.1); padding: .15rem .45rem; border-radius: 4px; border: 1px solid rgba(255, 113, 0, 0.25); }
  .percent-badge.ready-badge { color: #00ff66; background: rgba(0, 255, 100, 0.12); border-color: rgba(0, 255, 100, 0.35); }

  .goal-subinfo { display: flex; flex-wrap: wrap; gap: .8rem; font-size: .82rem; margin-bottom: .4rem; }
  .exp-text { color: var(--hud-muted); margin: 0; }
  .rolls-text { color: var(--hud-text); margin: 0; }

  .completable-box { background: rgba(0, 255, 100, 0.1); border: 1px solid rgba(0, 255, 100, 0.35); border-radius: 4px; padding: .4rem .6rem; margin: .5rem 0; }
  .ready { color: #00ff66; font-weight: 700; margin: 0; font-size: .88rem; }

  .missing-section { background: rgba(255, 85, 85, 0.05); border: 1px solid rgba(255, 85, 85, 0.2); border-radius: 4px; padding: .4rem .6rem; margin: .5rem 0; }
  .missing-list { list-style: none; padding: 0; margin: 0; font-size: .82rem; }
  .missing-list li { display: flex; justify-content: space-between; padding: .1rem 0; }
  .missing-name { color: var(--hud-text); }
  .missing-count { color: #ff5555; font-weight: 600; }

  .blockers-list { list-style: square; padding-left: 1.2rem; margin: .4rem 0; color: var(--hud-warning); font-size: .82rem; }

  .actions { display: flex; flex-wrap: wrap; gap: .4rem; margin-top: .6rem; }
  .btn-act { background: var(--hud-bg); color: var(--hud-text); border: 1px solid var(--hud-muted); padding: .25rem .5rem; border-radius: 3px; font-size: .8rem; cursor: pointer; }
  .btn-act:hover:not(:disabled) { border-color: var(--hud-primary); color: var(--hud-primary); }
  .btn-act:disabled { opacity: .4; cursor: not-allowed; }
  .btn-remove:hover:not(:disabled) { border-color: var(--hud-danger); color: var(--hud-danger); }

  .aggregates-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: .8rem; margin-top: 1rem; }
  .aggregate-card { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--hud-border); border-radius: 5px; padding: .6rem .8rem; }
  .aggregate-title { font-size: .85rem; font-weight: 700; color: var(--hud-primary); margin: 0 0 .4rem; }
  .aggregate-list { list-style: none; padding: 0; margin: 0; font-size: .8rem; }
  .aggregate-item { display: flex; justify-content: space-between; padding: .15rem 0; border-bottom: 1px solid rgba(255, 255, 255, 0.04); }
  .agg-name { color: var(--hud-muted); }
  .agg-count { font-weight: 600; }

  .pending-tag, .unverified-tag { color: var(--hud-muted); font-size: .75rem; margin-left: .3rem; }
  .muted { color: var(--hud-muted); font-size: .85rem; }
  .warning { color: var(--hud-warning); font-size: .85rem; }
  .error-msg { color: var(--hud-danger); font-size: .85rem; }
  .font-mono { font-family: var(--font-mono); }
</style>