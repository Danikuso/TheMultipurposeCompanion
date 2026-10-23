<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, cssPct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { MassacreTopic } from './types';

  const m = $derived(topic<MassacreTopic>('missions.massacre'));
  const piles = $derived(m?.piles ?? []);
  const pct = (k: number, n: number) => (n > 0 ? Math.min(100, (k / n) * 100) : 0);
</script>

<Panel titleKey="panel-missions-massacre" span={2}>
  {#if piles.length === 0}
    <p class="muted" data-testid="massacre-empty">{t('missions-massacre-none')}</p>
  {:else}
    {#each piles as p, i (p.target_faction + i)}
      <div class="pile" data-testid="massacre-pile">
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
