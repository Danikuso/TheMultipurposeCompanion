<!-- ui/src/lib/icons/GradeBadge.svelte -->
<script lang="ts">
  import { GRADES } from './index';

  interface Props {
    grade?: number | null;
    size?: number;
    class?: string;
  }

  const {
    grade = 1,
    size = 18,
    class: className = '',
  }: Props = $props();

  const svg = $derived(grade && GRADES[grade] ? GRADES[grade] : null);
</script>

{#if svg}
  <span
    class="grade-badge-icon {className}"
    style:width="{size}px"
    style:height="{size}px"
    title="Grado {grade}"
    aria-label="Grado {grade}"
  >
    {@html svg}
  </span>
{:else}
  <span class="grade-badge-fallback {className}">
    G{grade}
  </span>
{/if}

<style>
  .grade-badge-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    vertical-align: middle;
    color: var(--hud-primary, #ff7700);
  }

  .grade-badge-icon :global(svg) {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .grade-badge-fallback {
    font-weight: 700;
    color: var(--hud-primary, #ff7700);
  }
</style>
