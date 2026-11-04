<script module lang="ts">
  import { cards, type CardPlacement, type LayoutCtx } from './cards';
  import { customTab, themedTab } from './layoutStore.svelte';

  /** Disposición vigente (guardada o de fábrica) de una pestaña, sin ids desconocidos. */
  export function tabPlacements(tabKey: string): CardPlacement[] {
    const list = tabKey.startsWith('custom:')
      ? (customTab(tabKey.slice('custom:'.length))?.cards ?? [])
      : themedTab(tabKey).cards;
    return list.filter((p) => Object.hasOwn(cards, p.id));
  }

  /**
   * Las condiciones (`when`) solo rigen en las disposiciones. Las pestañas de módulo muestran todas sus
   * tarjetas como siempre (con sus estados vacíos y textos guía), también en edición.
   */
  export function isCardVisible(id: string, ctx: LayoutCtx, tabKey: string): boolean {
    if (tabKey.startsWith('module:')) return true;
    return cards[id]?.when?.(ctx) ?? true;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import CardFrame from './CardFrame.svelte';
  import { grid } from './gridColumns.svelte';
  import { onMount, untrack } from 'svelte';
  import { packLayout, visibleCards } from './layoutModel';
  import { applyOp, layoutState } from './layoutStore.svelte';
  import { FALLBACK_HEIGHT, boardGapPx, columnWidthPx, spanWidthPx } from './boardGeometry';

  let { tabKey, ctx, prefix }: { tabKey: string; ctx: LayoutCtx; prefix?: Snippet<[string]> } = $props();

  const placements = $derived(tabPlacements(tabKey));
  const shown = $derived(visibleCards(placements, (id) => isCardVisible(id, ctx, tabKey), layoutState.editing));

  let boardEl: HTMLElement | undefined = $state();
  let ro: ResizeObserver | null = $state.raw(null);
  // Ancho del tablero (0 = aún sin medir), separación y alto medido de cada marco (px).
  let boardWidth = $state(0);
  let gapPx = $state(boardGapPx());
  let heights = $state<Record<string, number>>({});

  /** Solo reasigna lo que cambia de verdad, para que medir nunca dispare ciclos de remedida. */
  function measureBoard() {
    if (!boardEl) return;
    const w = boardEl.clientWidth;
    if (w !== boardWidth) boardWidth = w;
    const g = boardGapPx();
    if (g !== gapPx) gapPx = g;
  }

  function measureFrames(frames: HTMLElement[]) {
    let next: Record<string, number> | null = null;
    for (const el of frames) {
      const id = el.getAttribute('data-layout-slot');
      if (!id) continue;
      const h = el.offsetHeight;
      if ((next ?? heights)[id] !== h) (next ??= { ...heights })[id] = h;
    }
    if (next) heights = next;
  }

  onMount(() => {
    measureBoard();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver((entries) => {
      const frames: HTMLElement[] = [];
      for (const e of entries) {
        if (e.target === boardEl) measureBoard();
        else if (e.target instanceof HTMLElement) frames.push(e.target);
      }
      measureFrames(frames);
    });
    ro = observer;
    return () => { observer.disconnect(); ro = null; };
  });

  // Mantiene observados exactamente el tablero y los marcos presentes y olvida el alto de los ausentes.
  // También vuelve a medir el ancho al cambiar la lista o entrar/salir de edición (p. ej. aparece una barra).
  const observed = new Set<Element>();
  $effect(() => {
    void shown;
    void layoutState.editing;
    const observer = ro;
    untrack(() => {
      if (!boardEl) return;
      measureBoard();
      const frames = Array.from(boardEl.querySelectorAll<HTMLElement>(':scope > [data-layout-slot]'));
      const present = new Set<Element>([boardEl, ...frames]);
      if (observer) {
        for (const el of present) if (!observed.has(el)) { observer.observe(el); observed.add(el); }
        for (const el of Array.from(observed)) if (!present.has(el)) { observer.unobserve(el); observed.delete(el); }
      }
      const ids = new Set(frames.map((f) => f.getAttribute('data-layout-slot')));
      const stale = Object.keys(heights).filter((id) => !ids.has(id));
      if (stale.length) {
        const next = { ...heights };
        for (const id of stale) delete next[id];
        heights = next;
      }
    });
  });

  const columns = $derived(grid.columns);
  const packed = $derived(packLayout(
    shown.map((s) => ({
      id: s.placement.id,
      x: s.placement.x,
      w: s.placement.w,
      height: heights[s.placement.id] ?? s.placement.h ?? FALLBACK_HEIGHT,
    })),
    columns, gapPx,
  ));
  const colWidth = $derived(columnWidthPx(boardWidth, columns, gapPx));
  const boxOf = (id: string) => {
    if (boardWidth <= 0) return null; // sin ancho medido (aún o jsdom): flujo normal, sin solapes
    const c = packed.cards.find((p) => p.id === id);
    if (!c) return null;
    return { left: c.col * (colWidth + gapPx), top: c.top, width: spanWidthPx(c.span, colWidth, gapPx) };
  };
  const geometry = () => ({ columns, gapPx, heights: $state.snapshot(heights) });
</script>

<div class="card-board" bind:this={boardEl} style:height={boardWidth > 0 ? `${packed.height}px` : undefined}>
  {#each shown as item, i (item.placement.id)}
    {#snippet framePrefix()}{@render prefix?.(item.placement.id)}{/snippet}
    <CardFrame placement={item.placement} card={cards[item.placement.id]} index={i} count={shown.length}
      ghost={item.ghost} editing={layoutState.editing} {columns} {ctx} prefix={framePrefix}
      box={boxOf(item.placement.id)}
      onop={(op) => applyOp(tabKey, placements.indexOf(item.placement), op, geometry())} />
  {/each}
</div>

<style>
  .card-board {
    grid-column: 1 / -1;
    position: relative;
    min-width: 0;
  }
</style>
