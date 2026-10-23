<!-- ui/src/lib/icons/SuperpowerIcon.svelte -->
<script lang="ts">
  import { SUPERPOWERS } from './index';

  interface Props {
    superpower?: string | null;
    size?: number;
    class?: string;
  }

  const {
    superpower = null,
    size = 18,
    class: className = '',
  }: Props = $props();

  const meta = $derived(superpower ? SUPERPOWERS[superpower] : null);
</script>

{#if meta}
  <span
    class="superpower-icon {className}"
    style:width="{size}px"
    style:height="{size}px"
    style:color={meta.color}
    title={meta.name}
    aria-label={meta.name}
  >
    {@html meta.svg}
  </span>
{:else if superpower}
  <span class="superpower-fallback {className}" style:font-size="{size * 0.75}px" title={superpower}>
    ★
  </span>
{/if}

<style>
  .superpower-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    vertical-align: middle;
  }

  .superpower-icon :global(svg) {
    width: 100%;
    height: 100%;
    object-fit: contain;
    filter: drop-shadow(0 0 3px currentColor);
  }

  .superpower-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--hud-muted);
  }
</style>
