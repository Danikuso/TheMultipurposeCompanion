<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from './lib/i18n';
  import { connect } from './lib/ws.svelte';
  import Grid from './shell/Grid.svelte';
  import ImportProgress from './shell/ImportProgress.svelte';
  import Panel from './shell/Panel.svelte';
  import Sidebar from './shell/Sidebar.svelte';
  import Toasts from './shell/Toasts.svelte';
  import TopBar from './shell/TopBar.svelte';

  let activeTab = $state('dashboard');

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
      <Panel titleKey="nav-dashboard">
        <p>{t('empty-no-data')}</p>
      </Panel>
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
