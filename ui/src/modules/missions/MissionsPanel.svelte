<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, decimal } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { createNow } from './now.svelte';
  import { countdown, isExpiringSoon } from './time';
  import { kindLabel, progressLabel } from './labels';
  import type { ActiveTopic } from './types';

  const a = $derived(topic<ActiveTopic>('missions.active'));
  const groups = $derived(a?.groups ?? []);
  const clock = createNow(
    () => a?.as_of,
    () => groups.flatMap((g) => g.missions.map((m) => m.expiry)),
  );

  function toRoute(system: string) {
    void action('missions.to_route', { system }).catch(() => {});
  }
</script>

<Panel titleKey="panel-missions-active" span={2}>
  {#if !a || groups.length === 0}
    <p class="muted" data-testid="missions-empty">{t('missions-none-active')}</p>
  {:else}
    {#each groups as g, gi (`${g.system}|${g.station}|${gi}`)}
      <div class="dest" class:next={g.next} data-testid="missions-group">
        <h5>
          {#if g.next}<span class="warn" aria-hidden="true">▶</span>{/if}
          <b class:warn={g.next}>{g.system ?? t('missions-no-destination')}</b>
          {#if g.station}· {g.station}{/if}
          {#if g.distance_ly != null}<span class="muted">· {t('mining-distance', { distance: decimal(g.distance_ly, 1) })}</span>{/if}
          {#if g.next && g.system}
            <span class="muted">· {t('missions-next-stop', { destination: g.system })}</span>
          {/if}
          {#if g.system}
            <button type="button" data-testid="mission-to-route" onclick={() => toRoute(g.system!)}>{t('missions-add-to-route')}</button>
          {/if}
        </h5>
        <table>
          <thead>
            <tr>
              <th>{t('missions-col-type')}</th><th>{t('missions-col-faction')}</th><th>{t('missions-col-progress')}</th>
              <th class="n">{t('missions-col-expiry')}</th><th class="n">{t('missions-col-reward')}</th>
            </tr>
          </thead>
          <tbody>
            {#each g.missions as m (m.id)}
              <tr data-testid="mission-row">
                <td>
                  {kindLabel(m.kind)}
                  {#if m.redirected}<span class="tag warn">{t('missions-redirected')}</span>{/if}
                </td>
                <td>{m.faction ?? '—'}</td>
                <td>
                  {#if m.details_missing}
                    <span class="muted" data-testid="mission-details-missing">{t('missions-details-missing')}</span>
                  {:else}
                    {progressLabel(m)}
                  {/if}
                </td>
                <td class="n mono">
                  {#if m.expiry}
                    <span
                      data-testid="mission-countdown"
                      class:expiring-soon={isExpiringSoon(m.expiry, clock.ms)}
                      title={t('missions-expires-in', { remaining: countdown(m.expiry, clock.ms) })}
                    >{countdown(m.expiry, clock.ms)}</span>
                  {:else}—{/if}
                </td>
                <td class="n mono">{m.reward == null ? '—' : credits(m.reward)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/each}
  {/if}
</Panel>

<style>
  .muted { color: var(--hud-muted); margin: 0.2rem 0; }
  .mono { font-family: var(--font-mono); }
  .dest { border-left: 3px solid var(--hud-border); padding: 0.2rem 0 0.2rem 0.6rem; margin: 0.5rem 0; }
  .dest.next { border-left-color: var(--hud-warning); }
  h5 { margin: 0 0 0.2rem; font-size: 1em; }
  h5 button { margin-left: 0.5rem; }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 0.15rem 0.5rem 0.15rem 0; white-space: nowrap; }
  th { color: var(--hud-muted); font-weight: normal; border-bottom: 1px solid var(--hud-border); }
  .n { text-align: right; }
  .warn { color: var(--hud-warning); }
  .expiring-soon { color: var(--hud-danger); font-weight: bold; }
  .tag { font-size: 0.75em; padding: 0 0.3rem; border: 1px solid currentColor; margin-left: 0.3rem; }
</style>
