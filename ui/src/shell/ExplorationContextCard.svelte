<script lang="ts">
  import SystemBodiesPanel from '../modules/exploration/SystemBodiesPanel.svelte';
  import ExobiologyPanel from '../modules/exobiology/ExobiologyPanel.svelte';
  import type { LayoutCtx } from './cards';
  import { contextPanel } from './layouts';

  let { ctx = { modules: [], manual: null } }: { ctx?: LayoutCtx } = $props();
  const shown = $derived(contextPanel(ctx.modules, ctx.manual));
</script>

{#if shown === 'exobiology'}
  <ExobiologyPanel />
{:else}
  <SystemBodiesPanel />
{/if}
