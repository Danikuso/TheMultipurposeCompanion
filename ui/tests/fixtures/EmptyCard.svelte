<script lang="ts">
  // Tarjeta que se oculta sola: no renderiza nada sin datos.
  const show = false;
</script>

{#if show}
  <p>visible</p>
{/if}
