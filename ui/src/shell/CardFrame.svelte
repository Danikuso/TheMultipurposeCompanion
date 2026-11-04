<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CardDef, CardPlacement, LayoutCtx } from './cards';
  import type { CardOp } from './layoutModel';
  import { effectiveWidth } from './layoutModel';
  import { setCardBox } from './cardBox';

  let {
    placement,
    card,
    ghost,
    editing,
    columns,
    prefix,
    ctx,
  }: {
    placement: CardPlacement;
    card: CardDef;
    index: number;
    count: number;
    ghost: boolean;
    editing: boolean;
    columns: number;
    onop?: (op: CardOp) => void;
    prefix?: Snippet;
    ctx: LayoutCtx;
  } = $props();

  setCardBox({
    get height() { return placement.h; },
    get editing() { return editing; },
  });

  const gridColumn = $derived(
    placement.w === 'full' ? '1 / -1' : `span ${effectiveWidth(placement.w, columns)}`,
  );
  const height = $derived.by(() => {
    if (placement.h === null) return undefined;
    return columns === 1 ? `min(${placement.h}px, max(240px, 70vh))` : `${placement.h}px`;
  });
  const Card = $derived(card.component);
</script>

<div class="card-frame" class:fixed={placement.h !== null} class:ghost class:editing
  style:grid-column={gridColumn} style:height={height} data-layout-slot={placement.id}
><div class="card-content">{@render prefix?.()}{#if !ghost}<Card {ctx} />{/if}</div></div>

<style>
  .card-frame {
    min-width: 0;
  }

  /* Componentes que se ocultan solos: fuera de edición el marco no ocupa sitio ni gap. */
  .card-frame:not(.editing):has(> .card-content:empty) {
    display: none;
  }

  .card-frame.fixed > .card-content {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
