<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children?: Snippet } = $props();
</script>

<main class="hud-grid">
  {@render children?.()}
</main>

<style>
  .hud-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.85rem;
    padding: 0.85rem;
    align-content: start;
    flex: 1;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .hud-grid::-webkit-scrollbar {
    width: 6px;
  }

  .hud-grid::-webkit-scrollbar-track {
    background: rgba(0, 0, 0, 0.15);
  }

  .hud-grid::-webkit-scrollbar-thumb {
    background: rgba(255, 113, 0, 0.25);
    border-radius: 4px;
  }

  .hud-grid::-webkit-scrollbar-thumb:hover {
    background: var(--hud-primary, #ff7100);
  }

  @media (min-width: 900px) {
    .hud-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (min-width: 1600px) {
    .hud-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (min-width: 2400px) {
    .hud-grid {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
</style>
