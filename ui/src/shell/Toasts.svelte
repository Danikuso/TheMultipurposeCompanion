<script lang="ts" module>
  import type { Alert } from '../lib/ws.svelte';

  export interface ToastItem {
    id: string;
    level: Alert['level'];
    text: string;
  }

  const toastItems = $state<ToastItem[]>([]);

  export function pushToast(text: string, level: Alert['level'] = 'Info', id?: string): void {
    const itemId = id ?? `toast-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const existingIdx = toastItems.findIndex((x) => x.id === itemId);
    if (existingIdx !== -1) {
      toastItems[existingIdx] = { id: itemId, level, text };
    } else {
      toastItems.push({ id: itemId, level, text });
    }
    if (level !== 'Critical') {
      setTimeout(() => {
        dismissToast(itemId);
      }, 8000);
    }
  }

  export function dismissToast(id: string): void {
    const idx = toastItems.findIndex((x) => x.id === id);
    if (idx !== -1) {
      toastItems.splice(idx, 1);
    }
  }
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n';
  import { onAlert } from '../lib/ws.svelte';

  onMount(() => {
    return onAlert((a) => {
      pushToast(t(a.key, a.args), a.level, a.id);
    });
  });
</script>

{#if toastItems.length > 0}
  <div class="toast-container" aria-live="polite">
    {#each toastItems as item (item.id)}
      <div class="toast toast-{item.level.toLowerCase()}">
        <span class="toast-text">{item.text}</span>
        <button type="button" class="toast-close" onclick={() => dismissToast(item.id)}>×</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 380px;
  }

  .toast {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.6rem 0.85rem;
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-left: 4px solid var(--hud-primary);
    border-radius: 4px;
    color: var(--hud-text);
  }

  .toast-warning {
    border-left-color: var(--hud-warning);
  }

  .toast-critical {
    border-left-color: var(--hud-danger);
    background: rgba(255, 43, 43, 0.15);
  }

  .toast-close {
    padding: 0.1rem 0.4rem;
    font-size: 1rem;
    line-height: 1;
  }
</style>
