<script lang="ts">
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { CarrierJumpTopic, CarrierJumpStatus } from './types';

  const jump = $derived(topic<CarrierJumpTopic>('carrier.jump'));
  let copied = $state(false);

  function formatTimer(totalSeconds: number | null | undefined): string {
    if (totalSeconds == null || totalSeconds < 0) return '00:00';
    const m = Math.floor(totalSeconds / 60);
    const s = totalSeconds % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }

  async function copyDestination(system: string) {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(system);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    }
  }

  const statusLabel = $derived.by(() => {
    switch (jump?.status) {
      case 'counting_down':
        return t('carrier-status-countdown');
      case 'locked':
        return t('carrier-status-lockdown');
      case 'cooldown':
        return t('carrier-status-cooldown');
      default:
        return t('carrier-status-idle');
    }
  });

  const statusBadgeClass = $derived.by(() => {
    switch (jump?.status) {
      case 'counting_down':
        return 'badge-orange';
      case 'locked':
        return 'badge-danger';
      case 'cooldown':
        return 'badge-cyan';
      default:
        return '';
    }
  });
</script>

<Panel titleKey="panel-carrier-jump" span={1}>
  {#if !jump}
    <p class="muted" data-testid="jump-no-data">{t('empty-no-data')}</p>
  {:else}
    <div class="jump-status-bar">
      <span class="metric-label">{t('carrier-jump-status')}</span>
      <span class="badge {statusBadgeClass}" data-testid="jump-status-badge">
        {statusLabel}
      </span>
    </div>

    {#if jump.status === 'counting_down' || jump.status === 'locked'}
      <div class="timer-card" data-testid="jump-countdown-card">
        <span class="metric-label">{t('carrier-jump-countdown')}</span>
        <div class="timer-digits mono danger" data-testid="jump-timer">
          {formatTimer(jump.seconds_remaining)}
        </div>
        <div
          class="hud-meter-track"
          role="progressbar"
          aria-valuenow={jump.seconds_remaining ?? 0}
          aria-valuemin={0}
          aria-valuemax={900}
        >
          <div
            class="hud-meter-fill fill-danger"
            style:width="{Math.min(100, Math.max(0, ((900 - (jump.seconds_remaining ?? 0)) / 900) * 100))}%"
          ></div>
        </div>
      </div>
    {:else if jump.status === 'cooldown'}
      <div class="timer-card" data-testid="jump-cooldown-card">
        <span class="metric-label">{t('carrier-cooldown-remaining')}</span>
        <div class="timer-digits mono cyan" data-testid="cooldown-timer">
          {formatTimer(jump.cooldown_seconds_remaining)}
        </div>
        <div
          class="hud-meter-track"
          role="progressbar"
          aria-valuenow={jump.cooldown_seconds_remaining}
          aria-valuemin={0}
          aria-valuemax={300}
        >
          <div
            class="hud-meter-fill fill-cyan"
            style:width="{Math.min(100, Math.max(0, ((300 - jump.cooldown_seconds_remaining) / 300) * 100))}%"
          ></div>
        </div>
      </div>
    {/if}

    <div class="destination-card">
      <div class="dest-info">
        <span class="metric-label">{t('carrier-jump-target')}</span>
        <span class="dest-name mono bold" data-testid="jump-dest-system">
          {jump.destination_system ?? '—'}
        </span>
        {#if jump.destination_body}
          <span class="dest-body sub" data-testid="jump-dest-body">{jump.destination_body}</span>
        {/if}
      </div>

      {#if jump.destination_system}
        <button
          type="button"
          class="copy-btn"
          data-testid="jump-copy-btn"
          onclick={() => copyDestination(jump.destination_system!)}
        >
          {copied ? t('carrier-copied') : t('carrier-copy-target')}
        </button>
      {/if}
    </div>
  {/if}
</Panel>

<style>
  .jump-status-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .timer-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem 0.85rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }
  .timer-digits {
    font-size: 2rem;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  .danger {
    color: var(--hud-danger);
  }
  .cyan {
    color: var(--hud-cyan);
  }
  .timer-card .hud-meter-track {
    width: 100%;
  }
  .destination-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.75rem 0.85rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .dest-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .dest-name {
    font-size: 1.1rem;
    color: var(--hud-text);
  }
  .sub {
    font-size: 0.75rem;
    color: var(--hud-muted);
  }
  .copy-btn {
    white-space: nowrap;
    font-size: 0.78rem;
    font-weight: 600;
  }
  .bold {
    font-weight: 700;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
