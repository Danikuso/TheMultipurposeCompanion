<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CardDef, CardPlacement, CardWidth, LayoutCtx } from './cards';
  import type { CardOp } from './layoutModel';
  import { H_STEP, effectiveWidth, snapHeight } from './layoutModel';
  import { startMoveDrag, startResizeDrag, type DragSession } from './pointerDrag';
  import { setCardBox } from './cardBox';
  import { t } from '../lib/i18n';

  let {
    placement,
    card,
    index,
    count,
    ghost,
    editing,
    columns,
    onop,
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

  // Vista previa del redimensionado: se aplica {w, h} temporalmente sin tocar la disposición.
  let preview = $state<{ w: CardWidth; h: number | null } | null>(null);
  let offset = $state<{ dx: number; dy: number } | null>(null);
  const shownW = $derived(preview?.w ?? placement.w);
  const shownH = $derived(preview ? preview.h : placement.h);

  const gridColumn = $derived(
    shownW === 'full' ? '1 / -1' : `span ${effectiveWidth(shownW, columns)}`,
  );
  const height = $derived.by(() => {
    if (shownH === null) return undefined;
    return columns === 1 ? `min(${shownH}px, max(240px, 70vh))` : `${shownH}px`;
  });
  const transform = $derived(offset ? `translate(${offset.dx}px, ${offset.dy}px)` : undefined);
  const Card = $derived(card.component);

  let frameEl: HTMLElement | undefined = $state();
  let contentEl: HTMLElement | undefined = $state();
  let emptyContent = $state(false);

  // En edición, un componente que se oculta solo (contenido vacío) se muestra como hueco atenuado.
  $effect(() => {
    if (!editing || !contentEl) { emptyContent = false; return; }
    const el = contentEl;
    const check = () => {
      emptyContent = !Array.from(el.childNodes)
        .some((n) => n.nodeType === Node.ELEMENT_NODE || (n.nodeType === Node.TEXT_NODE && n.textContent !== ''));
    };
    check();
    const mo = new MutationObserver(check);
    mo.observe(el, { childList: true, characterData: true, subtree: true });
    return () => mo.disconnect();
  });

  const showPlaceholder = $derived(editing && (ghost || emptyContent));
  const widthOptions = [1, 2, 3] as const;
  // La barra de controles es una capa superpuesta: el alto actual se mide sobre el contenido, sin ella.
  const currentHeight = () => placement.h ?? Math.round(contentEl?.getBoundingClientRect().height ?? 0);
  const emit = (op: CardOp) => onop?.(op);

  let session: DragSession | null = null;
  let marked: Element | null = null;
  const mark = (el: Element | null) => {
    if (marked === el) return;
    marked?.removeAttribute('data-drop-target');
    el?.setAttribute('data-drop-target', '');
    marked = el;
  };
  function finish() {
    session = null;
    preview = null;
    offset = null;
    mark(null);
  }

  function beginMove(e: PointerEvent) {
    if (!frameEl || !frameEl.parentElement || (e.pointerType === 'mouse' && e.button !== 0)) return;
    e.preventDefault();
    const frames = Array.from(frameEl.parentElement.children)
      .filter((c): c is HTMLElement => c instanceof HTMLElement && c.hasAttribute('data-layout-slot'));
    const handle = e.currentTarget as HTMLElement;
    session = startMoveDrag(e, handle, frames, index,
      (dx, dy, target) => { offset = { dx, dy }; mark(target === index ? null : frames[target] ?? null); },
      (target) => { finish(); if (target !== index) emit({ kind: 'move', to: target }); });
  }

  function beginResize(e: PointerEvent) {
    const parent = frameEl?.parentElement;
    if (!frameEl || !parent || (e.pointerType === 'mouse' && e.button !== 0)) return;
    e.preventDefault();
    const cs = getComputedStyle(parent);
    const gapPx = parseFloat(cs.columnGap) || 0;
    const inner = parent.getBoundingClientRect().width - (parseFloat(cs.paddingLeft) || 0) - (parseFloat(cs.paddingRight) || 0);
    const colWidthPx = Math.max(1, (inner - gapPx * (columns - 1)) / columns);
    session = startResizeDrag(e, e.currentTarget as HTMLElement,
      { w: placement.w, h: placement.h, heightPx: currentHeight(), widthPx: Math.round(frameEl.getBoundingClientRect().width) },
      { colWidthPx, gapPx, columns },
      (next) => { preview = next; },
      (next) => {
        finish();
        if (next.w !== placement.w) emit({ kind: 'width', w: next.w });
        if (next.h !== placement.h) emit({ kind: 'height', h: next.h });
      });
  }

  // Si se sale de la edición (o se destruye el marco) a mitad de arrastre, se aborta sin cambios.
  $effect(() => () => { session?.cancel(); finish(); });
  $effect(() => { if (!editing) { session?.cancel(); finish(); } });
</script>

<div class="card-frame" class:fixed={placement.h !== null} class:ghost class:editing class:dragging={offset !== null}
  style:grid-column={gridColumn} style:height={height} style:transform={transform}
  data-layout-slot={placement.id} bind:this={frameEl}
>{#if editing}<div class="card-controls" role="toolbar" aria-label={t(card.titleKey)}>
    <button type="button" class="drag-handle" data-op="drag" aria-label={t('layout-edit-drag')}
      title={t('layout-edit-drag')} onpointerdown={beginMove}>⠿</button>
    <span class="ctl-group">
      <button type="button" data-op="up" aria-label={t('layout-edit-move-up')} title={t('layout-edit-move-up')}
        disabled={index <= 0} onclick={() => emit({ kind: 'move', to: index - 1 })}>↑</button>
      <button type="button" data-op="down" aria-label={t('layout-edit-move-down')} title={t('layout-edit-move-down')}
        disabled={index >= count - 1} onclick={() => emit({ kind: 'move', to: index + 1 })}>↓</button>
    </span>
    <span class="ctl-group">
      {#each widthOptions as w (w)}
        <button type="button" data-op="width-{w}" aria-label={t(`layout-edit-width-${w}`)}
          title={t(`layout-edit-width-${w}`)} aria-pressed={placement.w === w}
          disabled={w > columns} onclick={() => emit({ kind: 'width', w })}>{w}</button>
      {/each}
      <button type="button" data-op="width-full" aria-label={t('layout-edit-width-full')}
        title={t('layout-edit-width-full')} aria-pressed={placement.w === 'full'}
        onclick={() => emit({ kind: 'width', w: 'full' })}>▭</button>
    </span>
    <span class="ctl-group">
      <button type="button" data-op="height-less" aria-label={t('layout-edit-height-less')}
        title={t('layout-edit-height-less')} onclick={() => emit({ kind: 'height', h: snapHeight(currentHeight() - H_STEP) })}>−</button>
      <button type="button" data-op="height-auto" aria-label={t('layout-edit-height-auto')}
        title={t('layout-edit-height-auto')} aria-pressed={placement.h === null}
        onclick={() => emit({ kind: 'height', h: null })}>auto</button>
      <button type="button" data-op="height-more" aria-label={t('layout-edit-height-more')}
        title={t('layout-edit-height-more')} onclick={() => emit({ kind: 'height', h: snapHeight(currentHeight() + H_STEP) })}>+</button>
    </span>
    <button type="button" class="remove" data-op="remove" aria-label={t('layout-edit-remove')}
      title={t('layout-edit-remove')} onclick={() => emit({ kind: 'remove' })}>⊖</button>
  </div><button type="button" class="resize-handle" data-op="resize" aria-label={t('layout-edit-resize')}
    title={t('layout-edit-resize')} onpointerdown={beginResize}>◢</button>{/if}<div class="card-content" class:dimmed={editing} inert={editing} bind:this={contentEl}
  >{@render prefix?.()}{#if !ghost}<Card {ctx} />{/if}</div>{#if showPlaceholder}<div class="card-placeholder">
    <span class="ph-title">{t(card.titleKey)}</span>
    <span class="ph-pill" title={t('layout-card-auto-hint')}>{t('layout-card-auto')}</span>
  </div>{/if}</div>

<style>
  /* El contenido rellena el marco (alto de la fila o alto fijo) para que las filas queden alineadas. */
  .card-frame {
    position: relative;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .card-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }

  .editing > .card-content:empty { flex: 0 0 auto; }
  .card-content.dimmed { opacity: 0.55; pointer-events: none; }

  .card-controls {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 5;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem 0.6rem;
    padding: 0.3rem 0.4rem;
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .card-frame.dragging { z-index: 20; opacity: 0.9; box-shadow: 0 6px 24px rgb(0 0 0 / 50%); }
  .card-frame:global([data-drop-target]) { outline: 2px solid var(--hud-primary); outline-offset: 2px; }

  .drag-handle, .resize-handle { touch-action: none; user-select: none; }
  .drag-handle { cursor: grab; }
  .resize-handle {
    position: absolute;
    right: 0;
    bottom: 0;
    z-index: 5;
    width: 1.6rem;
    height: 1.6rem;
    padding: 0;
    line-height: 1;
    cursor: nwse-resize;
    background: var(--hud-surface);
    color: var(--hud-primary);
    border: 1px solid var(--hud-border);
    border-radius: 4px 0 4px 0;
  }

  .ctl-group { display: inline-flex; flex-wrap: wrap; gap: 0.25rem; }

  .card-controls button {
    min-width: 1.9rem;
    padding: 0.2rem 0.45rem;
    line-height: 1.1;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 3px;
  }

  .card-controls button[aria-pressed='true'] { color: var(--hud-primary); border-color: var(--hud-primary); }
  .card-controls button:disabled { opacity: 0.4; cursor: not-allowed; }
  .card-controls .remove { margin-left: auto; color: var(--hud-danger, #ff2b2b); }

  .card-placeholder {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    min-height: 5.5rem;
    padding: 3rem 0.75rem 0.75rem;
    border: 1px dashed var(--hud-border);
    border-radius: 4px;
    opacity: 0.6;
  }

  .ph-title { color: var(--hud-text); }
  .ph-pill {
    padding: 0.1rem 0.5rem;
    border: 1px solid var(--hud-border);
    border-radius: 999px;
    font-size: 0.75rem;
    color: var(--hud-primary);
  }

  /* Componentes que se ocultan solos: fuera de edición el marco no ocupa sitio ni gap. */
  .card-frame:not(.editing):has(> .card-content:empty) {
    display: none;
  }
</style>
