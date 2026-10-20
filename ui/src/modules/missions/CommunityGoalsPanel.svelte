<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { createNow } from './now.svelte';
  import { countdown } from './time';
  import type { CommunityGoalsTopic } from './types';

  const c = $derived(topic<CommunityGoalsTopic>('missions.community_goals'));
  const goals = $derived(c?.goals ?? []);
  const clock = createNow(
    () => c?.as_of,
    () => goals.filter((g) => !g.complete).map((g) => g.expiry),
  );
</script>

<Panel titleKey="panel-missions-community-goals" span={2}>
  {#if goals.length === 0}
    <p class="muted" data-testid="cg-empty">{t('missions-cg-none')}</p>
  {:else}
    {#each goals as g (g.cgid)}
      <div class="goal" data-testid="cg-goal">
        <div>
          <b>{g.title}</b>
          {#if g.system}<span class="muted">· {g.system}{g.station ? ` · ${g.station}` : ''}</span>{/if}
          {#if g.complete}<span class="tag ok">{t('missions-cg-complete')}</span>{/if}
        </div>
        <div>{t('missions-cg-contribution', { contribution: integer(g.contribution) })}</div>
        {#if g.tier}<div>{t('missions-cg-tier', { tier: g.tier })}</div>{/if}
        {#if g.percentile != null}<div>{t('missions-cg-percentile', { percentile: integer(g.percentile) })}</div>{/if}
        {#if g.expiry && !g.complete}
          <div data-testid="missions-cg-remaining">{t('missions-cg-remaining', { remaining: countdown(g.expiry, clock.ms) })}</div>
        {/if}
        {#if g.bonus != null}<div>{t('missions-cg-reward', { reward: credits(g.bonus) })}</div>{/if}
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); }
  .goal { border-left: 3px solid var(--hud-border); padding-left: 0.6rem; margin: 0.5rem 0; }
  .tag { font-size: 0.75em; padding: 0 0.3rem; border: 1px solid currentColor; margin-left: 0.3rem; }
  .ok { color: var(--hud-success, var(--hud-primary)); }
</style>
