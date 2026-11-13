<script lang="ts">
  import { t } from '../../lib/i18n';
  import { copyKeyPage, KEY_PAGES, openSettingsAccounts, SERVICE_NAMES, type KeyedService } from './syncView';

  let {
    missingServices,
    onOpenSettings = openSettingsAccounts,
  }: {
    missingServices: KeyedService[];
    onOpenSettings?: () => void;
  } = $props();

  let copiedService = $state<KeyedService | null>(null);

  async function handleCopy(service: KeyedService) {
    const ok = await copyKeyPage(service);
    if (ok) {
      copiedService = service;
      setTimeout(() => {
        if (copiedService === service) copiedService = null;
      }, 2000);
    }
  }
</script>

{#if missingServices.length > 0}
  <section class="companion-unlock-guide" data-testid="sync-unlock-guide">
    <div class="guide-header">
      <div class="guide-title-row">
        <span class="guide-badge-pill">{t('accounts-sync-guide-badge')}</span>
        <span class="guide-summary">
          {missingServices.map((s) => SERVICE_NAMES[s]).join(' · ')}
        </span>
      </div>
      <p class="guide-desc">{t('accounts-sync-guide-desc')}</p>
    </div>

    <div class="guide-services-grid">
      {#each missingServices as svc (svc)}
        {@const serviceName = SERVICE_NAMES[svc]}
        <div class="guide-service-card" data-testid={`sync-guide-${svc}`}>
          <h4 class="guide-service-title">{t('accounts-sync-guide-title', { service: serviceName })}</h4>
          <ol class="hud-stepper guide-stepper">
            <li class="step-item active">
              <span class="step-num">1</span>
              <span class="step-text">{t('accounts-sync-guide-step-1', { service: serviceName })}</span>
            </li>
            <li class="step-item">
              <span class="step-num">2</span>
              <span class="step-text">{t('accounts-sync-guide-step-2', { service: serviceName })}</span>
            </li>
            <li class="step-item">
              <span class="step-num">3</span>
              <span class="step-text">{t('accounts-sync-guide-step-3')}</span>
            </li>
          </ol>

          <div class="guide-url mono" title={KEY_PAGES[svc]}>{KEY_PAGES[svc]}</div>

          <div class="form-actions">
            <button
              type="button"
              class="hud-btn btn-sm"
              onclick={() => handleCopy(svc)}
              data-testid={`guide-copy-${svc}`}
            >
              {copiedService === svc ? t('accounts-sync-guide-copied') : t('accounts-sync-guide-copy-link')}
            </button>
            <button
              type="button"
              class="hud-btn btn-sm"
              onclick={onOpenSettings}
              data-testid={`guide-settings-${svc}`}
            >
              {t('accounts-sync-guide-open-settings')}
            </button>
          </div>
        </div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .companion-unlock-guide {
    background: rgba(245, 158, 11, 0.06);
    border: 1px solid rgba(245, 158, 11, 0.35);
    border-left: 4px solid var(--hud-warning, #f59e0b);
    border-radius: 6px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .guide-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .guide-title-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .guide-badge-pill {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(245, 158, 11, 0.18);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .guide-summary {
    font-size: 12px;
    font-weight: 600;
    color: var(--hud-text, #f0f4f8);
  }

  .guide-desc {
    margin: 0;
    font-size: 12px;
    color: var(--hud-muted, #94a3b8);
    line-height: 1.4;
  }

  .guide-services-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 10px;
  }

  .guide-service-card {
    background: rgba(11, 17, 29, 0.65);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .guide-service-title {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: var(--hud-text, #f0f4f8);
  }

  .guide-stepper {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .guide-stepper .step-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    line-height: 1.35;
    color: var(--hud-text, #cbd5e1);
  }

  .guide-stepper .step-text {
    min-width: 0;
    word-break: break-word;
  }

  .guide-url {
    font-size: 11px;
    color: var(--hud-info, #38bdf8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .form-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 2px;
  }

  .form-actions :global(.hud-btn) {
    flex: 1 1 130px;
    justify-content: center;
  }
</style>
