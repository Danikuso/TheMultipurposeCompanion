<script lang="ts">
  import { t } from '../../lib/i18n';
  import { num1 } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { JumpTopic } from './types';

  const jump = $derived(topic<JumpTopic>('navigation.jump'));
  const known = $derived(jump != null && jump.source !== 'unknown');
</script>

<Panel titleKey="panel-jump" span={1}>
  {#if !known || !jump}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    <div class="row">
      <span class="label">{t('strip-range')}</span>
      <strong class="mono big" data-testid="jump-range">{num1(jump.range_ly)} ly</strong>
      {#if jump.supercharged}
        <span class="badge" data-testid="supercharged">{t('strip-supercharged')} ×{jump.supercharged}</span>
      {/if}
    </div>
    <div class="row">
      <span class="label">{t('strip-range-full')}</span>
      <span class="mono">{num1(jump.range_full_ly)} ly</span>
    </div>
    <div class="row">
      <span class="label">{t('jump-fuel')}</span>
      <span class="mono">{num1(jump.fuel_main)} / {num1(jump.fuel_capacity)} t</span>
    </div>
    <div class="row">
      <span class="label">{t('jump-max-fuel')}</span>
      <span class="mono">{num1(jump.max_fuel_per_jump)} t</span>
    </div>
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.4rem;
  }

  .label {
    color: var(--hud-muted);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .big {
    color: var(--hud-primary);
    font-size: var(--font-title-lg);
  }

  .badge {
    color: var(--hud-secondary);
    border: 1px solid var(--hud-secondary);
    border-radius: 3px;
    padding: 0 0.4rem;
    font-size: 0.85em;
  }
</style>
