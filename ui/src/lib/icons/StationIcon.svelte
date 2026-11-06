<!-- ui/src/lib/icons/StationIcon.svelte -->
<script lang="ts">
  import { t } from '../i18n';
  import { resolveStation } from './index';

  interface Props {
    stationType?: string | null;
    size?: number;
    class?: string;
  }

  const {
    stationType = null,
    size = 18,
    class: className = '',
  }: Props = $props();

  const meta = $derived(resolveStation(stationType));
  const name = $derived(meta.key === 'outpost' ? t('station-type-outpost') : meta.key === 'mega-ship' ? t('station-type-megaship') : meta.name);
</script>

{#if meta}
  <span
    class="station-icon {className}"
    style:width="{size}px"
    style:height="{size}px"
    title={name}
    aria-label={name}
  >
    {@html meta.svg}
  </span>
{/if}

<style>
  .station-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    vertical-align: middle;
    color: var(--hud-cyan, #00d9ff);
    opacity: 0.85;
  }

  .station-icon :global(svg) {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
</style>
