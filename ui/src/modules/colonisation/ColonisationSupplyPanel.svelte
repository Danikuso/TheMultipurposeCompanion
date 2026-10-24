<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { ColonisationDepotTopic } from './types';

  const depot = $derived(topic<ColonisationDepotTopic>('colonisation.depot'));
  const pendingResources = $derived(
    depot ? depot.resources.filter((r) => r.remaining > 0) : [],
  );

  let copiedItem = $state<string | null>(null);

  async function copyResource(name: string) {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(name);
      copiedItem = name;
      setTimeout(() => {
        copiedItem = null;
      }, 2000);
    }
  }
</script>

<Panel titleKey="panel-colonisation-supply" span={1}>
  {#if !depot || !depot.active_project}
    <p class="muted" data-testid="supply-no-data">{t('colonisation-no-depot')}</p>
  {:else if pendingResources.length === 0}
    <div class="complete-card" data-testid="supply-all-complete">
      <span class="badge badge-green">✔ COMPLETO</span>
      <p class="complete-msg">Todos los recursos del depósito han sido entregados.</p>
    </div>
  {:else}
    <div class="supply-list" data-testid="supply-list">
      {#each pendingResources as r (r.name)}
        <div class="supply-card" data-testid="supply-card">
          <div class="supply-info">
            <span class="supply-name bold" data-testid="supply-name">
              {r.name_localised ?? r.name}
            </span>
            <span class="supply-detail sub">
              Faltan <b class="gold mono">{integer(r.remaining)} t</b> (~{r.trips_remaining} viajes)
            </span>
          </div>

          <button
            type="button"
            class="copy-btn"
            data-testid="copy-resource-btn"
            onclick={() => copyResource(r.name_localised ?? r.name)}
          >
            {copiedItem === (r.name_localised ?? r.name)
              ? t('colonisation-copied')
              : 'Copiar'}
          </button>
        </div>
      {/each}
    </div>
  {/if}
</Panel>

<style>
  .complete-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    text-align: center;
  }
  .complete-msg {
    margin: 0;
    font-size: 0.85rem;
    color: var(--hud-text);
  }
  .supply-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .supply-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.65rem 0.75rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .supply-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .supply-name {
    font-size: 0.95rem;
    color: var(--hud-text);
  }
  .supply-detail {
    font-size: 0.75rem;
  }
  .copy-btn {
    font-size: 0.75rem;
    white-space: nowrap;
  }
  .sub {
    color: var(--hud-muted);
  }
  .bold {
    font-weight: 700;
  }
  .gold {
    color: #fbbf24;
  }
  .muted {
    color: var(--hud-muted);
    font-style: italic;
  }
</style>
