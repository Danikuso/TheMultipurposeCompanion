<!-- ui/src/lib/icons/ManufacturerIcon.svelte -->
<script lang="ts">
  import { resolveManufacturer } from './index';

  interface Props {
    shipType?: string | null;
    size?: number;
    class?: string;
  }

  const {
    shipType = null,
    size = 28,
    class: className = '',
  }: Props = $props();

  const meta = $derived(resolveManufacturer(shipType));
</script>

{#if meta}
  <span
    class="manufacturer-icon {className}"
    style:height="{size}px"
    title={meta.name}
    aria-label={meta.name}
  >
    {@html meta.svg}
  </span>
{/if}

<style>
  .manufacturer-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    vertical-align: middle;
    color: var(--hud-muted, #8ba0b8);
    opacity: 0.85;
    transition: opacity 0.2s, color 0.2s;
  }

  .manufacturer-icon:hover {
    opacity: 1;
    color: var(--hud-cyan, #00d9ff);
  }

  .manufacturer-icon :global(svg) {
    height: 100%;
    width: auto;
    max-width: 120px;
    object-fit: contain;
  }
</style>
