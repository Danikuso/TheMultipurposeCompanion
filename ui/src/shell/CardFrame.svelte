<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { CardDef, CardPlacement, LayoutCtx } from './cards';
  import type { CardOp } from './layoutModel';
  import { H_STEP, effectiveWidth, snapHeight } from './layoutModel';
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

  const gridColumn = $derived(
    placement.w === 'full' ? '1 / -1' : `span ${effectiveWidth(placement.w, columns)}`,
  );
  const height = $derived.by(() => {
    if (placement.h === null) return undefined;
    return columns === 1 ? `min(${placement.h}px, max(240px, 70vh))` : `${placement.h}px`;
  });
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
  const currentHeight = () => placement.h ?? Math.round(frameEl?.getBoundingClientRect().height ?? 0);
  const emit = (op: CardOp) => onop?.(op);
</script>

<div class="card-frame" class:fixed={placement.h !== null} class:ghost class:editing
  style:grid-column={gridColumn} style:height={height} data-layout-slot={placement.id} bind:this={frameEl}
>{#if editing}<div class="card-controls" role="toolbar" aria-label={t(card.titleKey)}>
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
  </div>{/if}<div class="card-content" class:dimmed={editing} inert={editing} bind:this={contentEl}
  >{@render prefix?.()}{#if !ghost}<Card {ctx} />{/if}</div>{#if showPlaceholder}<div class="card-placeholder">
    <span class="ph-title">{t(card.titleKey)}</span>
    <span class="ph-pill" title={t('layout-card-auto-hint')}>{t('layout-card-auto')}</span>
  </div>{/if}</div>

<style>
  /* El contenido rellena el marco (alto de la fila o alto fijo) para que las filas queden alineadas. */
  .card-frame {
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
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem 0.6rem;
    padding: 0.3rem 0.4rem;
    margin-bottom: 0.3rem;
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
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
    min-height: 3.5rem;
    padding: 0.75rem;
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
