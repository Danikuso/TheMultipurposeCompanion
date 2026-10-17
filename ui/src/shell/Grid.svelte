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
