<script lang="ts">
  import { onMount } from 'svelte';
  import { connect } from './lib/ws.svelte';
  import { modules } from './modules/registry';
  import Settings from './settings/Settings.svelte';
  import Grid from './shell/Grid.svelte';
  import ImportProgress from './shell/ImportProgress.svelte';
  import Sidebar from './shell/Sidebar.svelte';
  import Toasts from './shell/Toasts.svelte';
  import TopBar from './shell/TopBar.svelte';

  let activeTab = $state('dashboard');
  const activeModule = $derived(modules.find((m) => m.id === activeTab) ?? modules[0]);

  onMount(() => {
    connect();
  });
</script>

<div class="app-shell">
  <TopBar />
  <ImportProgress />
  <div class="app-body">
    <Sidebar {activeTab} onSelect={(id) => (activeTab = id)} />
    <Grid>
      {#if activeTab === 'settings'}
        <Settings />
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
</style>
