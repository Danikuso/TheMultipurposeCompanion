<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { RewardsTopic } from './types';

  const r = $derived(topic<RewardsTopic>('missions.rewards'));
  const materials = $derived(Object.entries(r?.materials ?? {}).sort(([a], [b]) => a.localeCompare(b)));
  const factions = $derived(Object.entries(r?.factions ?? {}).sort(([a], [b]) => a.localeCompare(b)));
  const signed = (n: number) => (n > 0 ? `+${n}` : `${n}`);
  const cls = (n: number) => (n > 0 ? 'ok' : n < 0 ? 'bad' : 'muted');
</script>

<Panel titleKey="panel-missions-rewards" span={1}>
  {#if !r}
    <p class="muted" data-testid="rewards-empty">{t('missions-no-data')}</p>
  {:else}
    <div data-testid="rewards-summary">
      {t('missions-rewards-credits', { credits: credits(r.credits) })}
      · {t('missions-rewards-completed', { completed: integer(r.completed) })}
      · {t('missions-rewards-failed', { count: integer(r.failed) })}
      · {t('missions-rewards-abandoned', { count: integer(r.abandoned) })}
    </div>
    {#if materials.length > 0}
      <div class="sub" data-testid="rewards-materials">
        <span class="muted">{t('missions-rewards-materials')}:</span>
        {materials.map(([n, c]) => `${c}× ${n}`).join(' · ')}
      </div>
    {/if}
    {#if factions.length > 0}
      <table data-testid="rewards-factions">
        <thead>
          <tr><th></th><th class="n">{t('missions-rewards-reputation')}</th><th class="n">{t('missions-rewards-influence')}</th></tr>
        </thead>
        <tbody>
          {#each factions as [name, f] (name)}
            <tr>
              <td>{name}</td>
              <td class="n mono {cls(f.reputation_score)}">{signed(f.reputation_score)}</td>
              <td class="n mono {cls(f.influence_score)}">{signed(f.influence_score)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); }
  .mono { font-family: var(--font-mono); }
  .sub { margin-top: 0.3rem; }
  table { border-collapse: collapse; margin-top: 0.4rem; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .ok { color: var(--hud-success, var(--hud-primary)); }
  .bad { color: var(--hud-danger); }
</style>
