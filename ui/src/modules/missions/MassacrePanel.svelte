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
        <div><b>{t('missions-massacre-target', { faction: p.target_faction })}</b></div>
        <div class="stack">
          {#each p.issuers as s, j (s.faction + j)}
            {@const done = s.kills >= s.needed}
            <span>{s.faction === '?' ? t('missions-issuer-unknown') : s.faction}{#if s.missions > 1} ({s.missions}){/if}</span>
            <div class="bar" data-testid="massacre-bar">
              <div
                role="progressbar"
                aria-label={s.faction}
                aria-valuemin="0"
                aria-valuemax={s.needed}
                aria-valuenow={s.kills}
                class="fill"
              ><i class:done style:width={cssPct(pct(s.kills, s.needed))}></i></div>
            </div>
            <span class="mono" class:ok={done}>{t('missions-progress', { done: s.kills, total: s.needed })}{done ? ' ✓' : ''}</span>
          {/each}
        </div>
        <div>
          <b class="warn" data-testid="massacre-remaining">{t('missions-massacre-missing', { kills: p.kills_remaining })}</b>
          · <span data-testid="massacre-reward">{t('missions-massacre-reward', { reward: credits(p.total_reward) })}</span>
        </div>
        {#if p.details_missing}<div class="muted small">{t('missions-details-missing')}</div>{/if}
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .small { font-size: 0.85em; }
  .mono { font-family: var(--font-mono); }
  .pile { margin-bottom: 0.8rem; }
  .stack { display: grid; grid-template-columns: auto 1fr auto; gap: 0.25rem 0.6rem; align-items: center; margin: 0.3rem 0; }
  .bar { height: 0.9rem; background: var(--hud-surface-alt); border: 1px solid var(--hud-border); }
  .fill { position: relative; height: 100%; }
  .fill i { position: absolute; left: 0; top: 0; bottom: 0; background: var(--hud-primary); }
  .fill i.done { background: var(--hud-success, var(--hud-primary)); }
  .warn { color: var(--hud-warning); }
  .ok { color: var(--hud-success, var(--hud-primary)); }
</style>
