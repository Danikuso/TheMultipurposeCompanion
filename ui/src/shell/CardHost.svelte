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
  import { dropTarget, packLayout, pinColumns, verticalNeighbor, visibleCards, type PackItem } from './layoutModel';
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
  const heightOf = (id: string, h: number | null) => heights[id] ?? h ?? FALLBACK_HEIGHT;
  const packItems = $derived<PackItem[]>(shown.map((s) => ({
    id: s.placement.id, x: s.placement.x, w: s.placement.w, height: heightOf(s.placement.id, s.placement.h),
  })));
  const packed = $derived(packLayout(packItems, columns, gapPx));
  const list = $derived(shown.map((s) => s.placement));
  const colWidth = $derived(columnWidthPx(boardWidth, columns, gapPx));

  // Arrastre en curso: columna e índice previstos (en la lista sin la tarjeta) y empaquetado de vista previa.
  let drag = $state<{ id: string; x: number; index: number } | null>(null);
  const previewPacked = $derived.by(() => {
    if (!drag) return null;
    const pinned = pinColumns(list, packed.cards);
    const byId = new Map(pinned.map((p) => [p.id, p.x]));
    const items = packItems.map((it) => ({ ...it, x: byId.get(it.id) ?? it.x }));
    const moving = items.find((it) => it.id === drag!.id);
    if (!moving) return null;
    const rest = items.filter((it) => it !== moving);
    rest.splice(drag.index, 0, { ...moving, x: drag.x });
    return packLayout(rest, columns, gapPx);
  });
  const shownPacked = $derived(previewPacked ?? packed);

  const boxFrom = (c: { col: number; top: number; span: number }) =>
    ({ left: c.col * (colWidth + gapPx), top: c.top, width: spanWidthPx(c.span, colWidth, gapPx) });
  const boxOf = (id: string) => {
    if (boardWidth <= 0) return null; // sin ancho medido (aún o jsdom): flujo normal, sin solapes
    // La tarjeta arrastrada conserva su sitio de origen: la desplaza el `transform` del marco.
    const c = (drag?.id === id ? packed : shownPacked).cards.find((p) => p.id === id);
    return c ? boxFrom(c) : null;
  };
  const placeholder = $derived.by(() => {
    if (!drag || !previewPacked || boardWidth <= 0) return null;
    const c = previewPacked.cards.find((p) => p.id === drag!.id);
    return c ? { ...boxFrom(c), height: c.height } : null;
  });
  const colOf = (id: string) => packed.cards.find((p) => p.id === id)?.col ?? 0;
  const geometry = () => ({ columns, gapPx, heights: $state.snapshot(heights) });

  function targetFor(id: string, dx: number, dy: number) {
    const own = packed.cards.find((p) => p.id === id);
    if (!own || boardWidth <= 0) return null;
    const box = boxFrom(own);
    return dropTarget({ id, span: own.span, leftPx: box.left + dx, topPx: box.top + dy }, list, packed.cards, colWidth, gapPx, columns);
  }
  function previewMove(id: string, dx: number, dy: number) {
    const t = targetFor(id, dx, dy);
    if (!t) return;
    if (drag?.id !== id || drag.x !== t.x || drag.index !== t.index) drag = { id, x: t.x, index: t.index };
  }
  const DRAG_MIN_PX = 4;
  function dropMove(id: string, dx: number, dy: number) {
    const t = targetFor(id, dx, dy);
    drag = null;
    // Soltar sin mover de verdad es una cancelación: no toca la disposición.
    if (!t || (Math.abs(dx) < DRAG_MIN_PX && Math.abs(dy) < DRAG_MIN_PX)) return;
    const from = placements.findIndex((p) => p.id === id);
    if (from < 0) return;
    // `to` se expresa sobre la lista de la pestaña sin la tarjeta (la misma que usa `applyCardOp`).
    const others = placements.filter((p) => p.id !== id);
    const before = list.filter((p) => p.id !== id)[t.index]; // tarjeta que quedará detrás
    const to = before ? others.findIndex((p) => p.id === before.id) : others.length;
    applyOp(tabKey, from, { kind: 'place', to, x: t.x }, geometry());
  }
  // Los índices se resuelven por id al aplicar cada operación: tras la primera, el borrador se clona y
  // los objetos del `{#each}` dejan de ser los de `placements` hasta el siguiente flush.
  const indexById = (id: string) => placements.findIndex((p) => p.id === id);
  function vertical(id: string, dir: -1 | 1) {
    const to = verticalNeighbor(list, packed.cards, list.findIndex((p) => p.id === id), dir);
    if (to !== null) applyOp(tabKey, indexById(id), { kind: 'move', to }, geometry());
  }
  const hasNeighbor = (id: string, dir: -1 | 1) => verticalNeighbor(list, packed.cards, list.findIndex((p) => p.id === id), dir) !== null;
</script>

<div class="card-board" bind:this={boardEl}
  style:height={boardWidth > 0 ? `${Math.max(packed.height, shownPacked.height)}px` : undefined}>
  {#each shown as item (item.placement.id)}
    {#snippet framePrefix()}{@render prefix?.(item.placement.id)}{/snippet}
    <CardFrame placement={item.placement} card={cards[item.placement.id]}
      ghost={item.ghost} editing={layoutState.editing} {columns} {ctx} prefix={framePrefix}
      box={boxOf(item.placement.id)} col={colOf(item.placement.id)}
      canUp={hasNeighbor(item.placement.id, -1)} canDown={hasNeighbor(item.placement.id, 1)}
      onvertical={(dir) => vertical(item.placement.id, dir)}
      onmovepreview={(dx, dy) => previewMove(item.placement.id, dx, dy)}
      onmovedrop={(dx, dy) => dropMove(item.placement.id, dx, dy)}
      onmovecancel={() => { drag = null; }}
      onop={(op) => applyOp(tabKey, indexById(item.placement.id), op, geometry())} />
  {/each}
  {#if placeholder}
    <div class="drop-placeholder" aria-hidden="true"
      style:left="{placeholder.left}px" style:top="{placeholder.top}px"
      style:width="{placeholder.width}px" style:height="{placeholder.height}px"></div>
  {/if}
</div>

<style>
  .card-board {
    grid-column: 1 / -1;
    position: relative;
    min-width: 0;
  }

  .drop-placeholder {
    position: absolute;
    box-sizing: border-box;
    pointer-events: none;
    border: 2px dashed var(--hud-primary);
    border-radius: 4px;
    background: rgba(255, 113, 0, 0.08);
    transition: left 120ms ease, top 120ms ease;
  }
</style>
