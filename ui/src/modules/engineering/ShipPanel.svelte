<script lang="ts">
  import { t } from '../../lib/i18n';
  import { integer, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import { engineeringActions, shipFlowAction, syncEngineeringContext } from './actions.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { blueprintLabel, engineeringDataLabel } from './labels';
  import type { EngineeringShipTopic, EngineeringFlowTopic } from './types';

  const data = $derived(topic<EngineeringShipTopic>('engineering.ship'));
  const selected = $derived(topic<EngineeringFlowTopic>('engineering.flow')?.slot);
  const identity = $derived(JSON.stringify([topic<{ name?: string }>('core.cmdr')?.name, data?.ship_id]));
  let failed = $state(false);
  let showList = $state(false);
  let generation = 0;

  $effect(() => {
    syncEngineeringContext(identity);
    generation++;
    failed = false;
  });

  const engineerableModules = $derived(
    (data?.modules ?? []).filter((row) => row.engineerable)
  );

  const activeModule = $derived(
    engineerableModules.find((row) => row.slot === selected) ?? engineerableModules[0]
  );

  async function select(slot: string) {
    if (engineeringActions.pending || slot === selected) return;
    const current = ++generation;
    failed = false;
    try {
      await shipFlowAction('select_module', { slot });
    } catch {
      if (current === generation) failed = true;
    }
  }
</script>

<Panel titleKey="panel-engineering-ship" span={2}>
  {#if failed}<p role="alert">{t('engineering-action-failed')}</p>{/if}
  {#if !engineerableModules.length}<p>{t('engineering-no-data')}</p>{/if}

  {#if engineerableModules.length}
    <div class="ship-controls">
      <div class="module-dropdown-wrapper">
        <label class="compact-selector-label" for="ship-module-select">
          <span class="label-text">Módulo ({engineerableModules.length}):</span>
          <select
            id="ship-module-select"
            class="glass-select"
            data-testid="ship-module-select"
            value={selected ?? activeModule?.slot ?? ''}
            onchange={(e) => select(e.currentTarget.value)}
            disabled={engineeringActions.pending}
          >
            {#each engineerableModules as row (row.slot)}
              {@const mod = row.module_type ? engineeringDataLabel('modtype', row.module_type) : { name: row.item, pending: false }}
              {@const bp = row.current ? blueprintLabel(row.current.blueprint) : null}
              <option value={row.slot}>
                {mod.name} · {row.slot}{#if bp} [G{row.current?.level} - {bp.name}]{:else} [{t('engineering-stock')}]{/if}
              </option>
            {/each}
          </select>
        </label>
      </div>

      <button
        type="button"
        class="glass-btn-secondary list-toggle-btn"
        data-testid="toggle-module-list"
        onclick={() => (showList = !showList)}
      >
        {showList ? '▲ Ocultar lista' : `▼ Ver lista (${engineerableModules.length})`}
      </button>
    </div>

    {#if activeModule}
      {@const activeMod = activeModule.module_type ? engineeringDataLabel('modtype', activeModule.module_type) : { name: activeModule.item, pending: false }}
      <div class="active-module-card">
        <div class="active-module-header">
          <span class="slot-badge">{activeModule.slot}</span>
          <span class="active-name">{activeMod.name}</span>
          {#if activeMod.pending}<small>{t('engineering-translation-pending')}</small>{/if}
        </div>
        <div class="active-module-meta">
          {#if activeModule.current}
            {@const bp = blueprintLabel(activeModule.current.blueprint)}
            <span class="meta-tag blueprint">{bp.name}</span>
            {#if bp.pending}<small>{t('engineering-translation-pending')}</small>{/if}
            <span class="meta-tag grade">{t('engineering-grade', { grade: integer(activeModule.current.level) })}</span>
            <span class="meta-tag quality">{t('engineering-col-quality')}: {pct(activeModule.current.quality)}</span>
            {#if activeModule.current.experimental}
              {@const exp = engineeringDataLabel('exp', activeModule.current.experimental)}
              <span class="meta-tag exp">{exp.name}</span>
              {#if exp.pending}<small>{t('engineering-translation-pending')}</small>{/if}
            {/if}
          {:else}
            <span class="meta-tag stock">{t('engineering-stock')}</span>
          {/if}
        </div>
      </div>
    {/if}

    <ul class="module-list" class:is-collapsed={!showList}>
      {#each engineerableModules as row (row.slot)}
        {@const module = row.module_type ? engineeringDataLabel('modtype', row.module_type) : { name: row.item, pending: false }}
        <li data-slot={row.slot} class:is-selected={selected === row.slot}>
          <button
            data-testid={`module-${row.slot}`}
            disabled={engineeringActions.pending || selected === row.slot}
            aria-pressed={selected === row.slot}
            onclick={() => select(row.slot)}
          >
            {module.name} · {row.slot}
          </button>
          {#if module.pending}<small>{t('engineering-translation-pending')}</small>{/if}
          {#if row.current}
            {@const blueprint = blueprintLabel(row.current.blueprint)}
            <span>{blueprint.name}</span>{#if blueprint.pending}<small>{t('engineering-translation-pending')}</small>{/if}
            <span>{t('engineering-grade', { grade: integer(row.current.level) })}</span>
            <span>{t('engineering-col-quality')}: {pct(row.current.quality)}</span>
            {#if row.current.experimental}
              {@const experimental = engineeringDataLabel('exp', row.current.experimental)}
              <span>{experimental.name}</span>{#if experimental.pending}<small>{t('engineering-translation-pending')}</small>{/if}
            {/if}
          {:else}
            <span>{t('engineering-stock')}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Panel>

<style>
  .ship-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--hud-border, rgba(0, 229, 255, 0.15));
  }
  .module-dropdown-wrapper {
    flex: 1;
    min-width: 240px;
  }
  .compact-selector-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: var(--hud-muted, #7ea0b8);
  }
  .label-text {
    white-space: nowrap;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.05em;
  }
  .glass-select {
    flex: 1;
    background: rgba(10, 16, 26, 0.9);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.3));
    color: var(--hud-primary, #00e5ff);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
    cursor: pointer;
  }
  .glass-select:focus {
    border-color: var(--hud-primary, #00e5ff);
    box-shadow: 0 0 8px rgba(0, 229, 255, 0.3);
  }
  .list-toggle-btn {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.25));
    color: var(--hud-text, #c9d7e5);
    padding: 0.35rem 0.65rem;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .list-toggle-btn:hover {
    background: rgba(0, 229, 255, 0.12);
    border-color: var(--hud-primary, #00e5ff);
    color: var(--hud-primary, #00e5ff);
  }
  .active-module-card {
    background: rgba(10, 18, 30, 0.7);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.2));
    border-left: 3px solid var(--hud-primary, #00e5ff);
    border-radius: 4px;
    padding: 0.6rem 0.8rem;
    margin-bottom: 0.75rem;
  }
  .active-module-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
  }
  .slot-badge {
    background: rgba(0, 229, 255, 0.12);
    border: 1px solid rgba(0, 229, 255, 0.3);
    color: var(--hud-primary, #00e5ff);
    font-size: 0.7rem;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    text-transform: uppercase;
    font-family: var(--font-mono, monospace);
  }
  .active-name {
    font-weight: 600;
    color: var(--hud-text-bright, #ffffff);
    font-size: 0.95rem;
  }
  .active-module-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }
  .meta-tag {
    font-size: 0.75rem;
    padding: 0.15rem 0.45rem;
    border-radius: 3px;
  }
  .meta-tag.blueprint {
    background: rgba(0, 229, 255, 0.1);
    color: var(--hud-primary, #00e5ff);
    border: 1px solid rgba(0, 229, 255, 0.25);
  }
  .meta-tag.grade {
    background: rgba(255, 170, 0, 0.15);
    color: #ffaa00;
    border: 1px solid rgba(255, 170, 0, 0.3);
    font-weight: 600;
  }
  .meta-tag.quality {
    color: var(--hud-muted, #7ea0b8);
    font-family: var(--font-mono, monospace);
  }
  .meta-tag.exp {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.3);
  }
  .meta-tag.stock {
    color: var(--hud-muted, #7ea0b8);
    font-style: italic;
  }
  .module-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .module-list.is-collapsed {
    display: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--hud-border, rgba(0, 229, 255, 0.1));
    overflow-wrap: anywhere;
  }
  li.is-selected {
    background: rgba(0, 229, 255, 0.05);
  }
  button {
    background: var(--hud-bg, #0b121e);
    border: 1px solid var(--hud-border, rgba(0, 229, 255, 0.2));
    color: var(--hud-primary, #00e5ff);
    padding: 0.3rem 0.5rem;
    border-radius: 3px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  small {
    color: var(--hud-muted, #7ea0b8);
  }
</style>


