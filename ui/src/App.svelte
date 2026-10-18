<script lang="ts">
  import { onMount } from 'svelte';
  import { connect, topic } from './lib/ws.svelte';
  import { t } from './lib/i18n';
  import { modules } from './modules/registry';
  import Settings from './settings/Settings.svelte';
  import Grid from './shell/Grid.svelte';
  import ImportProgress from './shell/ImportProgress.svelte';
  import Sidebar from './shell/Sidebar.svelte';
  import Toasts from './shell/Toasts.svelte';
  import TopBar from './shell/TopBar.svelte';
  import { autoLayout, contextPanel, layouts } from './shell/layouts';

  let userTab = $state<string | null>(null);
  let manual = $state<string | null>(null);
  let manualBody = $state<string | null>(null);
  const liveModules = $derived(topic<{ id: string; relevance: string }[]>('core.modules') ?? []);
  const location = $derived(topic<{ system?: string; system_address?: number; body?: string | null }>('core.location'));
  const exoBody = $derived(topic<{ system_address?: number; body_id?: number; body?: string | null }>('exobiology.body'));
  const matchingExoBody = $derived(!location || location.system_address === exoBody?.system_address ? exoBody : null);
  const bodyIdentity = $derived(JSON.stringify([
    location?.system_address ?? location?.system ?? matchingExoBody?.system_address,
    matchingExoBody?.body_id != null ? { id: matchingExoBody.body_id }
      : { name: location?.body ?? matchingExoBody?.body ?? null },
  ]));
  const activeTab = $derived(userTab
    ?? `layout:${autoLayout(liveModules, null) ?? 'exploration'}`);
  const activeLayout = $derived(layouts.find((layout) => activeTab === `layout:${layout.id}`));
  const layoutContext = $derived({ modules: liveModules, manual: manualBody === bodyIdentity ? manual : null });
  const visibleSlots = $derived(activeLayout?.slots.filter((slot) => !slot.when || slot.when(layoutContext)) ?? []);
  const selectedContext = $derived(contextPanel(liveModules, layoutContext.manual));
  const activeModule = $derived(modules.find((m) => m.id === activeTab) ?? modules[0]);

  $effect(() => {
    if (manualBody !== bodyIdentity) {
      manual = null;
      manualBody = bodyIdentity;
    }
  });

  function selectContext(id: string) {
    manualBody = bodyIdentity;
    manual = id;
  }

  onMount(() => {
    connect();
  });
</script>

<div class="app-shell">
  <TopBar />
  <ImportProgress />
  <div class="app-body">
    <Sidebar {activeTab} onSelect={(id) => (userTab = id)} />
    <Grid>
      {#if activeTab === 'settings'}
        <Settings />
      {:else if activeLayout}
        {#each visibleSlots as slot (slot.id)}
          <div class="layout-slot" class:full={slot.span === 'full'}
            style:--slot-span={slot.span === 'full' ? 1 : slot.span} data-layout-slot={slot.id}>
            {#if activeLayout.id === 'exploration' && slot.span === 2}
              <div class="context-controls" role="group" aria-label={t('layout-context')}>
                <button type="button" data-testid="context-bodies" aria-pressed={selectedContext === 'exploration'}
                  onclick={() => selectContext('exploration')}>{t('layout-context-bodies')}</button>
                <button type="button" data-testid="context-exobiology" aria-pressed={selectedContext === 'exobiology'}
                  onclick={() => selectContext('exobiology')}>{t('layout-context-exobiology')}</button>
              </div>
            {/if}
            <slot.panel />
          </div>
        {/each}
      {:else if activeModule}
        {#each activeModule.panels as PanelComp, idx (idx)}
          <PanelComp />
        {/each}
      {/if}
    </Grid>
  </div>
  <Toasts />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .app-body {
    display: flex;
    flex: 1;
  }

  .layout-slot { min-width: 0; grid-column: 1 / -1; }
  .context-controls { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; }
  .context-controls button { background: var(--hud-surface); color: var(--hud-text); border: 1px solid var(--hud-border); padding: 0.4rem 0.6rem; }
  .context-controls button[aria-pressed='true'] { color: var(--hud-primary); border-color: var(--hud-primary); }
  @media (min-width: 900px) {
    .layout-slot:not(.full) { grid-column: span var(--slot-span); }
  }
</style>
