<!-- ui/src/lib/icons/PowerIcon.svelte -->
<script lang="ts">
  import { POWERS } from './index';

  interface Props {
    power?: string | null;
    size?: number;
    allowToggle?: boolean;
    class?: string;
  }

  const {
    power = null,
    size = 48,
    allowToggle = true,
    class: className = '',
  }: Props = $props();

  const meta = $derived(power ? POWERS[power] : null);
  let showPortrait = $state(false);

  function toggle() {
    if (allowToggle && meta?.portrait) {
      showPortrait = !showPortrait;
    }
  }
</script>

{#if meta}
  <button
    type="button"
    class="power-avatar-btn {className}"
    style:--accent={meta.accentColor}
    style:--glow={meta.glowColor}
    style:width="{size}px"
    style:height="{size}px"
    onclick={toggle}
    title={meta.portrait ? 'Haz clic para alternar entre Emblema Vectorial y Retrato Oficial' : meta.name}
    aria-label={meta.name}
  >
    {#if showPortrait && meta.portrait}
      <img
        src={meta.portrait}
        alt={meta.name}
        class="portrait-img"
        style:width="{size}px"
        style:height="{size}px"
      />
    {:else}
      <div class="vector-seal" style:width="{size * 0.75}px" style:height="{size * 0.75}px">
        {@html meta.svg}
      </div>
    {/if}
  </button>
{:else}
  <div
    class="power-avatar-fallback {className}"
    style:width="{size}px"
    style:height="{size}px"
  >
    👑
  </div>
{/if}

<style>
  .power-avatar-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    background: radial-gradient(circle at center, rgba(16, 24, 40, 0.95), rgba(8, 12, 20, 0.98));
    border: 2px solid var(--accent, var(--hud-cyan));
    border-radius: 12px;
    box-shadow: 0 0 16px var(--glow, rgba(0, 217, 255, 0.35)), inset 0 0 8px rgba(0, 0, 0, 0.5);
    cursor: pointer;
    overflow: hidden;
    transition: all 0.25s ease;
    color: var(--accent, var(--hud-cyan));
  }

  .power-avatar-btn:hover {
    box-shadow: 0 0 24px var(--glow, rgba(0, 217, 255, 0.6)), inset 0 0 12px var(--glow, rgba(0, 217, 255, 0.2));
    transform: scale(1.03);
  }

  .portrait-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 10px;
    display: block;
  }

  .vector-seal {
    display: flex;
    align-items: center;
    justify-content: center;
    filter: drop-shadow(0 0 4px var(--glow, rgba(0, 217, 255, 0.5)));
  }

  .vector-seal :global(svg) {
    width: 100%;
    height: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .power-avatar-fallback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border);
    border-radius: 10px;
    font-size: 1.5rem;
  }
</style>
