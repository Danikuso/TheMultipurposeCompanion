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
  import { visibleCards } from './layoutModel';
  import { applyOp, layoutState } from './layoutStore.svelte';

  let { tabKey, ctx, prefix }: { tabKey: string; ctx: LayoutCtx; prefix?: Snippet<[string]> } = $props();

  const placements = $derived(tabPlacements(tabKey));
  const shown = $derived(visibleCards(placements, (id) => isCardVisible(id, ctx, tabKey), layoutState.editing));
</script>

{#each shown as item, i (item.placement.id)}
  {#snippet framePrefix()}{@render prefix?.(item.placement.id)}{/snippet}
  <CardFrame placement={item.placement} card={cards[item.placement.id]} index={i} count={shown.length}
    ghost={item.ghost} editing={layoutState.editing} columns={grid.columns} {ctx} prefix={framePrefix}
    onop={(op) => applyOp(tabKey, placements.indexOf(item.placement), op)} />
{/each}
