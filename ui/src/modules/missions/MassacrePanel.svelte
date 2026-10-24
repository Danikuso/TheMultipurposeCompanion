<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { MassacreTopic } from './types';

  const m = $derived(topic<MassacreTopic>('missions.massacre'));
  const piles = $derived(m?.piles ?? []);
  const pct = (k: number, n: number) => (n > 0 ? Math.min(100, (k / n) * 100) : 0);
  let selectedPileIndex = $state<number>(0);
  let showAllPiles = $state(false);
</script>

<Panel titleKey="panel-missions-massacre" span={2}>
  {#if piles.length === 0}
    <p class="muted" data-testid="massacre-empty">{t('missions-massacre-none')}</p>
  {:else}
    {#if piles.length > 1}
      <div class="massacre-controls">
        <label class="compact-selector-label" for="massacre-faction-select">
          <span class="label-text">Objetivo ({piles.length}):</span>
          <select
            id="massacre-faction-select"
            class="glass-select"
            data-testid="massacre-faction-select"
            bind:value={selectedPileIndex}
          >
            {#each piles as p, pi}
              <option value={pi}>
                {p.target_faction} · {p.kills_remaining} bajas restantes
              </option>
            {/each}
          </select>
        </label>
        <button
          type="button"
          class="glass-btn-secondary list-toggle-btn"
          data-testid="toggle-massacre-view"
          onclick={() => (showAllPiles = !showAllPiles)}
        >
          {showAllPiles ? '▲ Vista compacta' : `▼ Ver todos (${piles.length})`}
        </button>
      </div>
    {/if}

    {#each piles as p, i (p.target_faction + i)}
      <div
        class="pile"
        class:is-hidden={!showAllPiles && piles.length > 1 && i !== selectedPileIndex}
        data-testid="massacre-pile"
      >
        <div class="target-title"><b>{t('missions-massacre-target', { faction: p.target_faction })}</b></div>
        <div class="stack">
          {#each p.issuers as s, j (s.faction + j)}
            {@const done = s.kills >= s.needed}
            <span>{s.faction === '?' ? t('missions-issuer-unknown') : s.faction}{#if s.missions > 1} ({s.missions}){/if}</span>
            <div class="bar" data-testid="massacre-bar">
              <div
                role="progressbar"
                aria-label={s.faction === '?' ? t('missions-issuer-unknown') : s.faction}
                aria-valuemin="0"
                aria-valuemax={s.needed}
                aria-valuenow={s.kills}
                class="fill"
              ><i class:done style:width={cssPct(pct(s.kills, s.needed))}></i></div>
            </div>
            <span class="mono" class:ok={done}>{t('missions-progress', { done: s.kills, total: s.needed })}{done ? ' ✓' : ''}</span>
          {/each}
        </div>
        <div class="summary-line">
          <b class="warn" data-testid="massacre-remaining">{t('missions-massacre-missing', { kills: p.kills_remaining })}</b>
          · <span class="reward-val" data-testid="massacre-reward">{t('missions-massacre-reward', { reward: credits(p.total_reward) })}</span>
        </div>
        {#if p.details_missing}<div class="muted small">{t('missions-details-missing')}</div>{/if}
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted {
    color: var(--hud-muted);
    margin: 0.2rem 0;
  }
  .small {
    font-size: 0.78rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .pile {
    margin-bottom: 0.85rem;
    padding: 0.75rem 0.9rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--hud-border-subtle);
    border-radius: var(--radius-sm);
  }
  .pile.is-hidden {
    display: none;
  }
  .massacre-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 0.6rem;
    flex-wrap: wrap;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .compact-selector-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--hud-muted);
    flex: 1;
    min-width: 220px;
  }
  .compact-selector-label .label-text {
    white-space: nowrap;
    text-transform: uppercase;
    font-size: 0.72rem;
    letter-spacing: 0.05em;
  }
  .glass-select {
    flex: 1;
    background: rgba(10, 16, 26, 0.9);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.3));
    color: var(--hud-primary, #00e5ff);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
    outline: none;
    cursor: pointer;
  }
  .list-toggle-btn {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.25));
    color: var(--hud-text, #c9d7e5);
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .list-toggle-btn:hover {
    background: rgba(0, 229, 255, 0.12);
    border-color: var(--hud-primary, #00e5ff);
    color: var(--hud-primary, #00e5ff);
  }
  .target-title {
    font-size: 0.95rem;
    color: var(--hud-primary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 0.5rem;
  }
  .stack {
    display: grid;
    grid-template-columns: minmax(6rem, auto) 1fr auto;
    gap: 0.4rem 0.75rem;
    align-items: center;
    margin: 0.5rem 0;
    font-size: 0.86rem;
  }
  .bar {
    height: 0.7rem;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    overflow: hidden;
  }
  .fill {
    position: relative;
    height: 100%;
  }
  .fill i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: linear-gradient(90deg, #ff9933, var(--hud-primary));
    box-shadow: 0 0 6px rgba(255, 119, 0, 0.4);
    border-radius: 1px;
    transition: width var(--transition-fast);
  }
  .fill i.done {
    background: linear-gradient(90deg, #00cc66, var(--hud-ok));
    box-shadow: 0 0 6px rgba(68, 255, 136, 0.4);
  }
  .summary-line {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-top: 0.6rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--hud-border-subtle);
    font-size: 0.88rem;
  }
  .warn {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.1);
    border: 1px solid rgba(255, 170, 0, 0.25);
    padding: 0.15rem 0.5rem;
    border-radius: 2px;
  }
  .reward-val {
    color: var(--hud-cyan);
    font-weight: 600;
  }
  .ok {
    color: var(--hud-ok);
  }
</style>
