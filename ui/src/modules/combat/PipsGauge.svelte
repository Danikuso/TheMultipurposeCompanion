<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { Pips } from './types';

  let { pips }: { pips: Pips } = $props();

  const columns = $derived([
    { key: 'sys', value: pips.sys },
    { key: 'eng', value: pips.eng },
    { key: 'wep', value: pips.wep },
  ] as const);

  function state(value: number, i: number): 'full' | 'half' | 'empty' {
    if (value >= i + 1) return 'full';
    if (value >= i + 0.5) return 'half';
    return 'empty';
  }
</script>

<div class="pips" data-testid="pips">
  {#each columns as col (col.key)}
    <div class="col" data-col={col.key}>
      <div class="blocks">
        {#each [3, 2, 1, 0] as i (i)}
          <i data-state={state(col.value, i)}></i>
        {/each}
      </div>
      <span class="label">{t(`combat-pips-${col.key}`)}</span>
    </div>
  {/each}
</div>

<style>
  .pips { display: flex; gap: 1.2rem; }
  .col { display: flex; flex-direction: column; align-items: center; gap: 0.25rem; }
  .blocks { display: flex; flex-direction: column; gap: 3px; }
  i { display: block; width: 2rem; height: 0.7rem; border: 1px solid var(--hud-primary); }
  i[data-state='full'] { background: var(--hud-primary); }
  i[data-state='half'] { background: linear-gradient(to right, var(--hud-primary) 50%, transparent 50%); }
  .label { color: var(--hud-muted); font-size: 0.8em; }
</style>
