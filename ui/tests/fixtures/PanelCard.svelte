<script lang="ts">
  import Panel from '../../src/shell/Panel.svelte';
</script>

<Panel titleKey="panel-route" span={2}>
  <p>basic</p>
  {#snippet details()}<p>details</p>{/snippet}
</Panel>
