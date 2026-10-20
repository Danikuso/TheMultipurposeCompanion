<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';
  import { diag, diagOn } from '../lib/diag.svelte';
  let { items, rowHeight, height, row, label }: {
    items: T[]; rowHeight: number; height: number; row: Snippet<[T, number]>; label?: string;
  } = $props();
  let viewport: HTMLDivElement;
  let scrollTop = $state(0);
  const count = $derived(Math.ceil(height / rowHeight));
  const anchor = $derived(Math.min(Math.floor(scrollTop / rowHeight), Math.max(0, items.length - count)));
  const start = $derived(Math.max(0, anchor - 5));
  const end = $derived(Math.min(items.length, anchor + count + 5));
  $effect(() => {
    if (!diagOn()) return;
    diag('ui.list.render', { items: items.length, row_height: rowHeight, height, start, end });
  });
  $effect(() => {
    const maximum = Math.max(0, items.length * rowHeight - height);
    if (scrollTop > maximum) {
      scrollTop = maximum;
      viewport.scrollTop = maximum;
    }
  });
</script>

<!-- Scroll regions need keyboard focus so arrow/Page keys can reach offscreen rows. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div bind:this={viewport} data-testid="virtual-list" role="region" aria-label={label}
  tabindex="0" style:height="{height}px" style:overflow-y="auto"
  onscroll={() => scrollTop = viewport.scrollTop}>
  <div style:height="{items.length * rowHeight}px" style:position="relative">
    <div role="rowgroup" style:position="absolute" style:inset="0 0 auto"
      style:transform="translateY({start * rowHeight}px)">
      {#each items.slice(start, end) as item, index (start + index)}
        <div style:height="{rowHeight}px">{@render row(item, start + index)}</div>
      {/each}
    </div>
  </div>
</div>
