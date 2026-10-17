<script lang="ts">
  import { onMount } from 'svelte';
  import { credits, duration, ly } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  interface SessionTopic {
    started_at?: string | null;
    jumps?: number;
    distance_ly?: number;
    credits_at_start?: number | null;
    credits_now?: number | null;
    credits_delta?: number | null;
  }

  const session = $derived(topic<SessionTopic>('core.session'));
  let nowMs = $state(Date.now());

  onMount(() => {
    nowMs = Date.now();
    const timer = setInterval(() => {
      nowMs = Date.now();
    }, 30_000);
    return () => clearInterval(timer);
  });

  const elapsedMs = $derived.by(() => {
    if (!session?.started_at) return null;
    const start = Date.parse(session.started_at);
    if (Number.isNaN(start)) return null;
    return Math.max(0, nowMs - start);
  });

  const earnedCredits = $derived.by(() => {
    if (!session) return null;
    if (session.credits_delta != null) return session.credits_delta;
    if (session.credits_now != null && session.credits_at_start != null) {
      return session.credits_now - session.credits_at_start;
    }
    return 0;
  });
</script>

<Panel titleKey="panel-session">
  {#if !session || (!session.started_at && !session.jumps)}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="session-rows">
      <div class="row">
        <span class="label">{t('field-session-time')}</span>
        <strong class="mono" data-testid="session-duration">{duration(elapsedMs)}</strong>
      </div>
      <div class="row">
        <span class="label">{t('field-jumps')}</span>
        <span class="mono">{session.jumps ?? 0}</span>
      </div>
      <div class="row">
        <span class="label">{t('field-distance')}</span>
        <span class="mono">{ly(session.distance_ly ?? 0)}</span>
      </div>
      <div class="row">
        <span class="label">{t('field-credits-earned')}</span>
        <span class="mono">{credits(earnedCredits)}</span>
      </div>
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .session-rows {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .label {
    color: var(--hud-muted);
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
