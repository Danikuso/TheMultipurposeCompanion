<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import { resolveNavIcon } from '../lib/icons';
  import { CUSTOM_TAB_ICONS, NAME_MAX, normalizeTabName } from './layoutModel';

  let { mode, initial, onsubmit, ondelete, onclose }: {
    mode: 'create' | 'edit';
    initial?: { name: string; icon: string };
    onsubmit: (name: string, icon: string) => void;
    ondelete?: () => void;
    onclose: () => void;
  } = $props();

  const start = untrack(() => initial); // valores de partida: el formulario es dueño de su estado
  let name = $state(start?.name ?? '');
  let icon = $state(start?.icon && CUSTOM_TAB_ICONS.includes(start.icon) ? start.icon : CUSTOM_TAB_ICONS[0]);
  let dialogEl = $state<HTMLElement>();
  let nameEl = $state<HTMLInputElement>();

  const valid = $derived(normalizeTabName(name) !== null);
  const title = $derived(t(mode === 'create' ? 'layout-custom-new-title' : 'layout-custom-rename-title'));

  function submit(e: Event) {
    e.preventDefault();
    const n = normalizeTabName(name);
    if (n !== null) onsubmit(n, icon);
  }

  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !dialogEl) return;
    const items = Array.from(dialogEl.querySelectorAll<HTMLElement>('button:not(:disabled), input'));
    if (items.length === 0) return;
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || !dialogEl.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !dialogEl.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }

  onMount(() => {
    const opener = document.activeElement as HTMLElement | null;
    nameEl?.focus();
    nameEl?.select();
    // Captura en window: Esc cierra el diálogo y no llega al listener que cancela la edición.
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.stopPropagation();
      e.preventDefault();
      onclose();
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      if (opener?.isConnected) opener.focus();
    };
  });
</script>

<!-- Cierra con pointerdown sobre el propio fondo: soltar una selección encima no debe cerrarlo. -->
<div class="tab-backdrop" role="presentation" data-testid="custom-tab-backdrop"
  onpointerdown={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="tab-dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1"
    data-testid="custom-tab-dialog" bind:this={dialogEl} onkeydown={trapTab}>
  <form class="tab-form" onsubmit={submit}>
    <header class="tab-head">
      <h3>{title}</h3>
      <button type="button" class="tab-close" aria-label={t('layout-custom-close')} onclick={onclose}>✕</button>
    </header>
    <label class="tab-field">
      <span>{t('layout-custom-name')}</span>
      <input type="text" data-testid="custom-tab-name" bind:this={nameEl} bind:value={name}
        maxlength={NAME_MAX} autocomplete="off" />
    </label>
    <div class="tab-field" role="group" aria-label={t('layout-custom-icon')}>
      <span>{t('layout-custom-icon')}</span>
      <div class="tab-icons">
        {#each CUSTOM_TAB_ICONS as id (id)}
          <button type="button" class="tab-icon" data-icon={id} aria-pressed={icon === id}
            aria-label={t(`nav-${id}`)} title={t(`nav-${id}`)} onclick={() => (icon = id)}>
            <span class="hud-svg-icon" aria-hidden="true">{@html resolveNavIcon(id)}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="tab-actions">
      <button type="submit" class="primary" data-testid="custom-tab-submit" disabled={!valid}>
        {t(mode === 'create' ? 'layout-custom-create' : 'layout-custom-save')}
      </button>
      <button type="button" data-testid="custom-tab-cancel" onclick={onclose}>{t('layout-edit-cancel')}</button>
      {#if mode === 'edit' && ondelete}
        <button type="button" class="danger" data-testid="custom-tab-dialog-delete" onclick={ondelete}>
          {t('layout-custom-delete')}
        </button>
      {/if}
    </div>
  </form>
  </div>
</div>

<style>
  .tab-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(4, 8, 16, 0.78);
    backdrop-filter: blur(8px);
  }

  .tab-dialog {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    box-sizing: border-box;
    width: 100%;
    max-width: 380px;
    max-height: 100%;
    padding: 0.85rem;
    background: rgba(10, 16, 26, 0.94);
    border: 1px solid var(--hud-primary);
    border-radius: 8px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.7);
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 170, 0, 0.3) transparent;
  }

  .tab-form { display: flex; flex-direction: column; gap: 0.8rem; }

  .tab-head { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .tab-head h3 { margin: 0; font-size: 1rem; color: var(--hud-primary); text-transform: uppercase; letter-spacing: 0.04em; }

  .tab-close {
    flex-shrink: 0;
    padding: 0.2rem 0.5rem;
    background: transparent;
    color: var(--hud-text-muted, #7890a8);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .tab-field { display: flex; flex-direction: column; gap: 0.35rem; font-size: 0.78rem; color: var(--hud-text-muted, #7890a8); }

  .tab-field input {
    box-sizing: border-box;
    width: 100%;
    padding: 0.4rem 0.6rem;
    background: rgba(6, 10, 18, 0.8);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .tab-icons { display: flex; flex-wrap: wrap; gap: 0.4rem; }

  .tab-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 36px;
    height: 36px;
    padding: 0;
    background: transparent;
    color: var(--hud-text-muted, #7890a8);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    cursor: pointer;
  }

  .tab-icon .hud-svg-icon { display: inline-flex; width: 20px; height: 20px; }
  .tab-icon .hud-svg-icon :global(svg) { width: 100%; height: 100%; display: block; }
  .tab-icon:hover, .tab-icon:focus-visible { border-color: var(--hud-primary); color: var(--hud-text); }
  .tab-icon[aria-pressed='true'] { color: var(--hud-primary); border-color: var(--hud-primary); background: rgba(255, 113, 0, 0.15); }

  .tab-actions { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .tab-actions button { flex: 1; min-width: 6rem; padding: 0.4rem 0.7rem; }
  .tab-actions button:disabled { opacity: 0.45; cursor: default; }
  .tab-actions .primary { color: var(--hud-primary); border-color: var(--hud-primary); }
  .tab-actions .danger { color: var(--hud-danger); border-color: var(--hud-danger); }
</style>
