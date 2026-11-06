<script lang="ts">
  import { t } from '../lib/i18n';
  import { updater, installUpdate, formatBytes } from '../lib/updater.svelte';

  function close() {
    if (!updater.status.is_downloading) {
      updater.isOpenModal = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && updater.isOpenModal && !updater.status.is_downloading) {
      close();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if updater.isOpenModal}
  <div class="update-backdrop" role="presentation" onclick={close}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="update-dialog hud-panel"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <header class="update-header">
        <div class="update-title-wrap">
          <span class="update-icon" aria-hidden="true">🚀</span>
          <h3>{t('update-modal-title')}</h3>
        </div>
        {#if !updater.status.is_downloading}
          <button
            type="button"
            class="btn-close"
            aria-label={t('ui-close')}
            onclick={close}
          >✕</button>
        {/if}
      </header>

      <div class="update-body">
        <div class="version-badge-row">
          <span class="badge badge-cyan">v{updater.status.current_version || '1.0.1'}</span>
          <span class="arrow-sep" aria-hidden="true">➔</span>
          <span class="badge badge-gold">v{updater.status.latest_version}</span>
          {#if updater.status.asset_size}
            <span class="asset-size">({formatBytes(updater.status.asset_size)})</span>
          {/if}
        </div>

        {#if updater.status.release_notes}
          <div class="release-notes-box">
            <h4 class="notes-title">{t('update-modal-release-notes')}</h4>
            <div class="notes-content">{updater.status.release_notes}</div>
          </div>
        {/if}

        {#if updater.status.is_downloading || updater.status.download_progress > 0}
          <div class="progress-section">
            <div class="progress-label-row">
              <span class="progress-status">{updater.status.status_message || t('update-modal-downloading')}</span>
              <span class="progress-pct font-mono">{Math.round(updater.status.download_progress * 100)}%</span>
            </div>
            <div class="hud-meter-track">
              <div
                class="hud-meter-fill fill-cyan"
                style="width: {Math.max(0, Math.min(100, updater.status.download_progress * 100))}%;"
              ></div>
            </div>
            <p class="download-hint">{t('update-modal-downloading-hint')}</p>
          </div>
        {/if}

        {#if updater.status.error}
          <div class="update-error-card">
            <span class="error-icon" aria-hidden="true">⚠️</span>
            <span class="error-text">{updater.status.error}</span>
          </div>
        {/if}
      </div>

      <footer class="update-footer">
        {#if !updater.status.is_downloading}
          <button
            type="button"
            class="btn btn-secondary"
            onclick={close}
          >
            {t('update-modal-later-btn')}
          </button>
          <button
            type="button"
            class="btn btn-primary btn-update"
            onclick={() => { void installUpdate(); }}
          >
            {t('update-modal-install-btn')}
          </button>
        {:else}
          <button type="button" class="btn btn-secondary" disabled>
            {updater.status.status_message || t('update-modal-downloading')}
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .update-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(4, 8, 16, 0.78);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    animation: fadeIn 0.2s ease-out;
  }

  .update-dialog {
    width: 100%;
    max-width: 540px;
    background: rgba(10, 16, 26, 0.94);
    border: 1px solid rgba(255, 170, 0, 0.4);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.7), 0 0 24px rgba(255, 170, 0, 0.15);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .update-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.15rem;
    background: rgba(255, 170, 0, 0.08);
    border-bottom: 1px solid rgba(255, 170, 0, 0.25);
  }

  .update-title-wrap {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .update-title-wrap h3 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: #ffaa00;
    text-transform: uppercase;
  }

  .update-icon {
    font-size: 1.25rem;
  }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--hud-text-muted, #7890a8);
    font-size: 1.1rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    line-height: 1;
    transition: color 0.15s, background 0.15s;
  }

  .btn-close:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.1);
  }

  .update-body {
    padding: 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .version-badge-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }

  .arrow-sep {
    color: var(--hud-text-muted, #7890a8);
    font-weight: bold;
  }

  .asset-size {
    font-size: 0.85rem;
    color: var(--hud-text-muted, #7890a8);
    font-family: var(--font-mono, monospace);
  }

  .release-notes-box {
    background: rgba(6, 10, 18, 0.8);
    border: 1px solid rgba(80, 120, 160, 0.2);
    border-radius: 6px;
    padding: 0.85rem;
  }

  .notes-title {
    margin: 0 0 0.5rem 0;
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--hud-text-muted, #7890a8);
  }

  .notes-content {
    max-height: 180px;
    overflow-y: auto;
    font-size: 0.88rem;
    line-height: 1.45;
    color: #d8e2ec;
    white-space: pre-wrap;
    word-break: break-word;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 170, 0, 0.3) transparent;
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    background: rgba(0, 195, 255, 0.05);
    border: 1px solid rgba(0, 195, 255, 0.25);
    border-radius: 6px;
    padding: 0.75rem 0.85rem;
  }

  .progress-label-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
  }

  .progress-status {
    color: #00c3ff;
  }

  .progress-pct {
    font-weight: bold;
    color: #ffffff;
  }

  .download-hint {
    margin: 0.3rem 0 0 0;
    font-size: 0.75rem;
    color: var(--hud-text-muted, #7890a8);
    line-height: 1.35;
  }

  .update-error-card {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 0.85rem;
    background: rgba(255, 59, 48, 0.1);
    border: 1px solid rgba(255, 59, 48, 0.35);
    border-radius: 6px;
    color: #ff5555;
    font-size: 0.85rem;
  }

  .update-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 0.85rem 1.15rem;
    background: rgba(6, 10, 18, 0.6);
    border-top: 1px solid rgba(80, 120, 160, 0.2);
  }

  .btn-update {
    background: linear-gradient(135deg, #ffaa00, #ff8800);
    color: #000000;
    font-weight: 600;
    border: 1px solid #ffcc00;
  }

  .btn-update:hover {
    background: linear-gradient(135deg, #ffbb11, #ff9900);
    box-shadow: 0 0 12px rgba(255, 170, 0, 0.4);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
