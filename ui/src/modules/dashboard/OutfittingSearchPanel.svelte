<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from '../../lib/i18n';
  import { credits, decimal, integer, ly } from '../../lib/format';
  import { formatShipName } from '../../lib/ships';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type {
    ModuleCategory,
    OutfittingCatalogTopic,
    OutfittingModuleEntry,
    OutfittingModuleQuery,
    OutfittingSearchTopic,
    OutfittingStationRow,
    PadSizeLabel,
    WeaponMode,
  } from './outfittingTypes';

  interface Suggestion {
    name: string;
    id64: number;
  }

  const MAX_BASKET_MODULES = 10;
  const AUTOCOMPLETE_DEBOUNCE_MS = 300;
  const AUTOCOMPLETE_MIN_CHARS = 3;

  const catalog = $derived(topic<OutfittingCatalogTopic>('ship.outfitting_catalog'));
  const searchState = $derived(topic<OutfittingSearchTopic>('ship.outfitting_search'));

  const ships = $derived.by(() => {
    const raw = catalog?.ships ?? [];
    return [...raw].sort((a, b) =>
      formatShipName(a.symbol).localeCompare(formatShipName(b.symbol)),
    );
  });
  const modulesCatalog = $derived(catalog?.modules ?? []);
  const classes = $derived(catalog?.classes ?? [0, 1, 2, 3, 4, 5, 6, 7, 8]);
  const ratings = $derived(catalog?.ratings ?? ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I']);

  const currentSystem = $derived(catalog?.commander?.current_system ?? '');
  const pledgedPower = $derived(catalog?.commander?.pledged_power ?? null);
  const hasEliteRank = $derived(catalog?.commander?.has_elite_rank ?? false);

  // Origen con sincronización reactiva mientras el piloto no lo edite manualmente
  let originText = $state('');
  let originTouched = $state(false);
  let suggestions = $state<Suggestion[]>([]);
  let suggestionsOpen = $state(false);
  let suggestSeq = 0;
  let suggestTimer: ReturnType<typeof setTimeout> | null = null;
  let suggestCtrl: AbortController | null = null;

  $effect(() => {
    if (!originTouched && currentSystem) {
      originText = currentSystem;
    }
  });

  // Cesta: 1 nave opcional + hasta 10 componentes
  let selectedShip = $state('');
  let basketModules = $state<OutfittingModuleQuery[]>([]);

  // Constructor de componente
  let categoryFilter = $state<'all' | ModuleCategory>('all');
  let moduleFilterText = $state('');
  let selectedModuleName = $state('');
  let selectedClass = $state('');
  let selectedRating = $state('');
  let selectedWeaponMode = $state<'' | WeaponMode>('');
  let selectedArmourShip = $state('');

  const filteredModules = $derived.by(() => {
    const q = moduleFilterText.trim().toLowerCase();
    return modulesCatalog.filter((m) => {
      if (categoryFilter !== 'all' && m.category !== categoryFilter) return false;
      if (q && !m.name.toLowerCase().includes(q)) return false;
      return true;
    });
  });

  const activeModuleEntry = $derived<OutfittingModuleEntry | null>(
    modulesCatalog.find((m) => m.name === selectedModuleName) ?? null,
  );

  $effect(() => {
    if (!activeModuleEntry?.has_weapon_mode) {
      selectedWeaponMode = '';
    }
    if (!activeModuleEntry?.is_armour) {
      selectedArmourShip = '';
    }
  });

  // Filtros de ayudante
  let padMode = $state<'auto' | 'S' | 'M' | 'L'>('auto');
  let onlyLyrDiscount = $state(false);
  let includeOwnStronghold = $state(false);
  let excludePermits = $state(true);
  let permitsTouched = $state(false);
  let includePlanetary = $state(true);
  let includeCarriers = $state(false);

  $effect(() => {
    if (!permitsTouched) {
      excludePermits = !hasEliteRank;
    }
  });

  $effect(() => {
    if (!pledgedPower && includeOwnStronghold) {
      includeOwnStronghold = false;
    }
  });

  // Estado UI de resultados
  let expandedRows = $state<Record<number, boolean>>({});
  let copiedKey = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => {
    if (suggestTimer !== null) clearTimeout(suggestTimer);
    if (copyTimer !== null) clearTimeout(copyTimer);
    suggestCtrl?.abort();
  });

  const basketCount = $derived((selectedShip ? 1 : 0) + basketModules.length);
  const status = $derived(searchState?.status ?? 'idle');
  const isLoading = $derived(status === 'loading');
  const stations = $derived(searchState?.stations ?? []);
  const fallbackRows = $derived(searchState?.fallback ?? []);
  const hiddenPermitCount = $derived(searchState?.hidden_permit_count ?? 0);

  async function fetchOriginSuggestions(query: string, seq: number) {
    suggestCtrl?.abort();
    const ctrl = new AbortController();
    suggestCtrl = ctrl;
    try {
      const res = await fetch(
        `/api/navigation/autocomplete?q=${encodeURIComponent(query)}&page=0`,
        { signal: ctrl.signal },
      );
      if (seq !== suggestSeq || !res.ok) return;
      const body = (await res.json()) as { items?: Suggestion[] };
      if (seq === suggestSeq) {
        suggestions = body.items ?? [];
        suggestionsOpen = suggestions.length > 0;
      }
    } catch {
      if (seq === suggestSeq) suggestions = [];
    }
  }

  function onOriginInput(ev: Event) {
    originTouched = true;
    originText = (ev.currentTarget as HTMLInputElement).value;
    suggestSeq += 1;
    suggestCtrl?.abort();
    if (suggestTimer !== null) {
      clearTimeout(suggestTimer);
      suggestTimer = null;
    }
    const q = originText.trim();
    if (q.length < AUTOCOMPLETE_MIN_CHARS) {
      suggestions = [];
      suggestionsOpen = false;
      return;
    }
    const seq = suggestSeq;
    suggestTimer = setTimeout(() => {
      suggestTimer = null;
      void fetchOriginSuggestions(q, seq);
    }, AUTOCOMPLETE_DEBOUNCE_MS);
  }

  function pickSuggestion(s: Suggestion) {
    suggestSeq += 1;
    originTouched = true;
    originText = s.name;
    suggestions = [];
    suggestionsOpen = false;
  }

  function closeSuggestionsLater() {
    setTimeout(() => {
      suggestionsOpen = false;
    }, 150);
  }

  function useCurrentSystem() {
    originTouched = false;
    originText = currentSystem;
    suggestions = [];
    suggestionsOpen = false;
  }

  function addModuleToBasket() {
    if (!selectedModuleName || basketModules.length >= MAX_BASKET_MODULES) return;
    const item: OutfittingModuleQuery = {
      name: selectedModuleName,
    };
    if (selectedClass !== '') item.class = Number(selectedClass);
    if (selectedRating !== '') item.rating = selectedRating;
    if (activeModuleEntry?.has_weapon_mode && selectedWeaponMode !== '') {
      item.weapon_mode = selectedWeaponMode;
    }
    if (activeModuleEntry?.is_armour && selectedArmourShip !== '') {
      item.armour_ship = selectedArmourShip;
    }
    basketModules = [...basketModules, item];
  }

  function removeModuleAt(idx: number) {
    basketModules = basketModules.filter((_, i) => i !== idx);
  }

  function formatModulePill(m: OutfittingModuleQuery): string {
    const prefix = `${m.class ?? ''}${m.rating ?? ''}`.trim();
    const base = prefix ? `${prefix} ${m.name}` : m.name;
    const extras: string[] = [];
    if (m.weapon_mode === 'Fixed') extras.push(t('outfitting-mount-fixed'));
    else if (m.weapon_mode === 'Gimbal') extras.push(t('outfitting-mount-gimbal'));
    else if (m.weapon_mode === 'Turret') extras.push(t('outfitting-mount-turret'));
    if (m.armour_ship) extras.push(formatShipName(m.armour_ship));
    return extras.length > 0 ? `${base} (${extras.join(' · ')})` : base;
  }

  function submitSearch(ev?: Event) {
    ev?.preventDefault();
    if (basketCount === 0 || isLoading) return;
    expandedRows = {};
    void action('ship.outfitting_search', {
      reference_system: originText.trim(),
      ship: selectedShip || null,
      modules: basketModules,
      pad_mode: padMode,
      only_lyr_discount: onlyLyrDiscount,
      include_own_stronghold: includeOwnStronghold && Boolean(pledgedPower),
      exclude_permits: excludePermits,
      include_planetary: includePlanetary,
      include_carriers: includeCarriers,
    }).catch(() => {});
  }

  function clearAll() {
    selectedShip = '';
    basketModules = [];
    expandedRows = {};
    void action('ship.outfitting_clear', {}).catch(() => {});
  }

  function toggleBreakdown(idx: number) {
    expandedRows = { ...expandedRows, [idx]: !expandedRows[idx] };
  }

  async function copySystem(systemName: string, key: string) {
    try {
      await navigator.clipboard?.writeText(systemName);
      copiedKey = key;
      if (copyTimer !== null) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copiedKey = null;
      }, 1800);
    } catch {
      // portapapeles no disponible
    }
  }

  function plotRoute(systemName: string) {
    if (!systemName) return;
    void action('navigation.plan', {
      to: systemName,
      mode: 'galaxy',
      use_ship: true,
    }).catch(() => {});
  }

  function padShort(pad: PadSizeLabel | null): string {
    if (pad === 'Large') return 'L';
    if (pad === 'Medium') return 'M';
    if (pad === 'Small') return 'S';
    return '—';
  }

  function formatLs(ls: number | null): string {
    if (ls == null || Number.isNaN(ls)) return '—';
    return `${integer(Math.round(ls))} ls`;
  }

  function formatUpdatedAge(iso: string | null): string {
    if (!iso) return '—';
    const ts = Date.parse(iso);
    if (Number.isNaN(ts)) return '—';
    const refMs = searchState?.fetched_at ? Date.parse(searchState.fetched_at) : Date.now();
    const baseMs = Number.isNaN(refMs) ? Date.now() : refMs;
    const hours = Math.max(0, Math.floor((baseMs - ts) / 3_600_000));
    if (hours < 1) return t('outfitting-age-now');
    if (hours < 48) return t('outfitting-age-hours', { count: hours });
    return t('outfitting-age-days', { count: Math.floor(hours / 24) });
  }

  function affordabilityBadge(st: OutfittingStationRow): {
    cls: string;
    label: string;
    tooltip: string;
  } | null {
    if (st.affordability === 'ok') {
      return {
        cls: 'badge-green',
        label: t('outfitting-afford-ok'),
        tooltip: t('outfitting-afford-ok-tooltip'),
      };
    }
    if (st.affordability === 'tight_rebuy') {
      return {
        cls: 'badge-amber',
        label: t('outfitting-afford-tight'),
        tooltip: t('outfitting-afford-tight-tooltip'),
      };
    }
    if (st.affordability === 'insufficient') {
      return {
        cls: 'badge-danger',
        label: t('outfitting-afford-insufficient'),
        tooltip: t('outfitting-afford-insufficient-tooltip'),
      };
    }
    return null;
  }
</script>

<Panel titleKey="panel-ship-outfitting" span={2}>
  <div class="outfitting-root" data-testid="outfitting-panel">
    <form class="search-form" onsubmit={submitSearch}>
      <!-- Fila 1: Sistema de partida + Nave opcional -->
      <div class="top-grid">
        <div class="field-group origin-group">
          <label class="field-label" for="outfitting-origin">{t('outfitting-origin-label')}</label>
          <div class="origin-input-row">
            <div class="autocomplete-wrap">
              <input
                id="outfitting-origin"
                data-testid="outfitting-origin-input"
                type="text"
                class="hud-input"
                value={originText}
                placeholder={t('outfitting-origin-placeholder')}
                autocomplete="off"
                oninput={onOriginInput}
                onblur={closeSuggestionsLater}
                onkeydown={(ev) => {
                  if (ev.key === 'Escape') suggestionsOpen = false;
                }}
              />
              {#if suggestionsOpen && suggestions.length > 0}
                <ul class="suggestions-list" role="listbox">
                  {#each suggestions as s (s.id64)}
                    <li>
                      <button
                        type="button"
                        class="suggestion-item"
                        onmousedown={() => pickSuggestion(s)}
                      >
                        {s.name}
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
            <button
              type="button"
              class="hud-btn btn-sm use-current-btn"
              data-testid="outfitting-use-current-btn"
              disabled={!currentSystem}
              title={currentSystem
                ? t('outfitting-use-current-tooltip', { system: currentSystem })
                : ''}
              onclick={useCurrentSystem}
            >
              {t('outfitting-use-current')}
            </button>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label" for="outfitting-ship">{t('outfitting-ship-label')}</label>
          <select
            id="outfitting-ship"
            data-testid="outfitting-ship-select"
            class="hud-select"
            bind:value={selectedShip}
          >
            <option value="">{t('outfitting-ship-none')}</option>
            {#each ships as s (s.name)}
              <option value={s.name}>{formatShipName(s.symbol)} [{s.pad}]</option>
            {/each}
          </select>
        </div>
      </div>

      <!-- Fila 2: Constructor de componentes -->
      <div class="module-builder-card">
        <div class="builder-header">
          <span class="field-label">{t('outfitting-module-label', { max: MAX_BASKET_MODULES })}</span>
          <div class="cat-pills" role="group">
            {#each [
              ['all', 'outfitting-cat-all'],
              ['core_internal', 'outfitting-cat-core'],
              ['optional_internal', 'outfitting-cat-optional'],
              ['hardpoint', 'outfitting-cat-hardpoint'],
              ['utility', 'outfitting-cat-utility'],
              ['armour', 'outfitting-cat-armour'],
            ] as [catId, labelKey] (catId)}
              <button
                type="button"
                class="cat-pill"
                class:active={categoryFilter === catId}
                onclick={() => (categoryFilter = catId as 'all' | ModuleCategory)}
              >
                {t(labelKey)}
              </button>
            {/each}
          </div>
        </div>

        <div class="builder-controls">
          <input
            type="text"
            class="hud-input filter-input"
            bind:value={moduleFilterText}
            placeholder={t('outfitting-module-search-placeholder')}
          />

          <select
            data-testid="outfitting-module-select"
            class="hud-select module-select"
            bind:value={selectedModuleName}
            aria-label={t('outfitting-module-select-placeholder')}
          >
            <option value="">{t('outfitting-module-select-placeholder')}</option>
            {#each filteredModules as mod (mod.name)}
              <option value={mod.name}>{mod.name}</option>
            {/each}
          </select>

          <label class="compact-spec">
            <span>{t('outfitting-class-label')}</span>
            <select
              data-testid="outfitting-class-select"
              class="hud-select spec-select"
              bind:value={selectedClass}
            >
              <option value="">{t('outfitting-class-any')}</option>
              {#each classes as cls (cls)}
                <option value={String(cls)}>{cls}</option>
              {/each}
            </select>
          </label>

          <label class="compact-spec">
            <span>{t('outfitting-rating-label')}</span>
            <select
              data-testid="outfitting-rating-select"
              class="hud-select spec-select"
              bind:value={selectedRating}
            >
              <option value="">{t('outfitting-rating-any')}</option>
              {#each ratings as r (r)}
                <option value={r}>{r}</option>
              {/each}
            </select>
          </label>

          {#if activeModuleEntry?.has_weapon_mode}
            <label class="compact-spec">
              <span>{t('outfitting-mount-label')}</span>
              <select
                data-testid="outfitting-mount-select"
                class="hud-select mount-select"
                bind:value={selectedWeaponMode}
              >
                <option value="">{t('outfitting-mount-any')}</option>
                <option value="Fixed">{t('outfitting-mount-fixed')}</option>
                <option value="Gimbal">{t('outfitting-mount-gimbal')}</option>
                <option value="Turret">{t('outfitting-mount-turret')}</option>
              </select>
            </label>
          {/if}

          {#if activeModuleEntry?.is_armour}
            <label class="compact-spec">
              <span>{t('outfitting-armour-ship-label')}</span>
              <select
                data-testid="outfitting-armour-ship-select"
                class="hud-select mount-select"
                bind:value={selectedArmourShip}
              >
                <option value="">{t('outfitting-armour-ship-any')}</option>
                {#each ships as s (s.name)}
                  <option value={s.name}>{formatShipName(s.symbol)}</option>
                {/each}
              </select>
            </label>
          {/if}

          <button
            type="button"
            class="hud-btn add-module-btn"
            data-testid="outfitting-add-module-btn"
            disabled={!selectedModuleName || basketModules.length >= MAX_BASKET_MODULES}
            onclick={addModuleToBasket}
          >
            {t('outfitting-add-btn')}
          </button>
        </div>
      </div>

      <!-- Fila 3: Cesta activa -->
      <div class="basket-box" data-testid="outfitting-basket">
        <span class="field-label">{t('outfitting-basket-label', { count: basketCount })}</span>
        {#if basketCount === 0}
          <span class="basket-empty">{t('outfitting-basket-empty')}</span>
        {:else}
          <div class="basket-pills">
            {#if selectedShip}
              <span class="basket-pill ship-pill">
                <span class="pill-kind">{t('outfitting-basket-ship-tag')}</span>
                <span class="pill-text">{formatShipName(selectedShip)}</span>
                <button
                  type="button"
                  class="pill-remove"
                  aria-label={t('outfitting-remove-item', { name: formatShipName(selectedShip) })}
                  onclick={() => (selectedShip = '')}
                >
                  ×
                </button>
              </span>
            {/if}
            {#each basketModules as mod, idx (`${mod.name}-${idx}`)}
              {@const pillLabel = formatModulePill(mod)}
              <span class="basket-pill">
                <span class="pill-text">{pillLabel}</span>
                <button
                  type="button"
                  class="pill-remove"
                  aria-label={t('outfitting-remove-item', { name: pillLabel })}
                  onclick={() => removeModuleAt(idx)}
                >
                  ×
                </button>
              </span>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Fila 4: Plataforma e interruptores Glass HUD (incl. Stronghold de tu potencia) -->
      <div class="filters-bar">
        <div class="pad-selector" role="group" aria-label={t('outfitting-pad-label')}>
          <span class="pad-label">{t('outfitting-pad-label')}:</span>
          {#each ['auto', 'S', 'M', 'L'] as mode (mode)}
            <button
              type="button"
              class="pad-btn"
              class:active={padMode === mode}
              title={mode === 'auto' ? t('outfitting-pad-auto-tooltip') : ''}
              onclick={() => (padMode = mode as 'auto' | 'S' | 'M' | 'L')}
            >
              {mode === 'auto' ? t('outfitting-pad-auto') : mode}
            </button>
          {/each}
        </div>

        <div class="switches-wrap">
          <button
            type="button"
            role="switch"
            aria-checked={onlyLyrDiscount}
            data-testid="outfitting-switch-lyr"
            class="hud-switch-pill"
            class:on={onlyLyrDiscount}
            title={t('outfitting-filter-lyr-tooltip')}
            onclick={() => (onlyLyrDiscount = !onlyLyrDiscount)}
          >
            <span class="hud-switch-track"><span class="hud-switch-thumb"></span></span>
            <span>{t('outfitting-filter-lyr')}</span>
          </button>

          <button
            type="button"
            role="switch"
            aria-checked={includeOwnStronghold}
            data-testid="outfitting-switch-stronghold"
            class="hud-switch-pill stronghold-switch"
            class:on={includeOwnStronghold}
            disabled={!pledgedPower}
            title={pledgedPower
              ? t('outfitting-filter-stronghold-tooltip', { faction: pledgedPower })
              : t('outfitting-filter-stronghold-unpledged-tooltip')}
            onclick={() => {
              if (pledgedPower) includeOwnStronghold = !includeOwnStronghold;
            }}
          >
            <span class="hud-switch-track"><span class="hud-switch-thumb"></span></span>
            <span>
              {pledgedPower
                ? t('outfitting-filter-stronghold-power', { faction: pledgedPower })
                : t('outfitting-filter-stronghold')}
            </span>
          </button>

          <button
            type="button"
            role="switch"
            aria-checked={excludePermits}
            data-testid="outfitting-switch-permits"
            class="hud-switch-pill"
            class:on={excludePermits}
            title={t('outfitting-filter-exclude-permits-tooltip')}
            onclick={() => {
              permitsTouched = true;
              excludePermits = !excludePermits;
            }}
          >
            <span class="hud-switch-track"><span class="hud-switch-thumb"></span></span>
            <span>{t('outfitting-filter-exclude-permits')}</span>
          </button>

          <button
            type="button"
            role="switch"
            aria-checked={includePlanetary}
            data-testid="outfitting-switch-planetary"
            class="hud-switch-pill"
            class:on={includePlanetary}
            title={t('outfitting-filter-planetary-tooltip')}
            onclick={() => (includePlanetary = !includePlanetary)}
          >
            <span class="hud-switch-track"><span class="hud-switch-thumb"></span></span>
            <span>{t('outfitting-filter-planetary')}</span>
          </button>

          <button
            type="button"
            role="switch"
            aria-checked={includeCarriers}
            data-testid="outfitting-switch-carriers"
            class="hud-switch-pill"
            class:on={includeCarriers}
            title={t('outfitting-filter-carriers-tooltip')}
            onclick={() => (includeCarriers = !includeCarriers)}
          >
            <span class="hud-switch-track"><span class="hud-switch-thumb"></span></span>
            <span>{t('outfitting-filter-carriers')}</span>
          </button>
        </div>
      </div>

      <!-- Fila 5: Fila dedicada de acciones del formulario -->
      <div class="form-actions">
        <button
          type="submit"
          class="hud-btn primary-action-btn"
          data-testid="outfitting-submit-btn"
          disabled={basketCount === 0 || isLoading}
        >
          {isLoading ? t('outfitting-searching-btn') : t('outfitting-search-btn')}
        </button>
        <button
          type="button"
          class="hud-btn secondary-action-btn"
          data-testid="outfitting-clear-btn"
          disabled={basketCount === 0 && status === 'idle'}
          onclick={clearAll}
        >
          {t('outfitting-clear-btn')}
        </button>
      </div>
    </form>

    <!-- Mensajes de estado y resultados -->
    {#if hiddenPermitCount > 0}
      <div class="permit-notice" data-testid="outfitting-hidden-permits">
        {t('outfitting-hidden-permits-notice', { count: hiddenPermitCount })}
      </div>
    {/if}

    {#if status === 'loading'}
      <div class="status-banner">{t('outfitting-status-loading')}</div>
    {:else if status === 'offline'}
      <div class="status-banner warn">{t('outfitting-status-offline')}</div>
    {:else if status === 'unknown_origin'}
      <div class="status-banner warn">
        {t('outfitting-status-unknown-origin', {
          system: searchState?.params?.reference_system ?? originText,
        })}
      </div>
    {:else if status === 'missing_origin'}
      <div class="status-banner warn">{t('outfitting-status-missing-origin')}</div>
    {:else if status === 'error'}
      <div class="status-banner danger">{t('outfitting-status-error')}</div>
    {:else if status === 'no_data'}
      <div class="status-banner">{t('outfitting-status-no-data')}</div>
    {:else if status === 'ok' && stations.length > 0}
      <div class="results-header">
        <span class="results-summary">{t('outfitting-results-count', { count: stations.length })}</span>
      </div>

      <div
        class="stations-list"
        class:scrollable={stations.length > 6}
        data-testid="outfitting-results-list"
      >
        {#each stations as st, idx (`${st.system_name}-${st.station_name}-${idx}`)}
          {@const afford = affordabilityBadge(st)}
          {@const copyId = `hit-${idx}`}
          <div class="station-card" class:highlight={idx === 0}>
            <div class="station-main-row">
              <div class="station-identity">
                <div class="station-title-line">
                  <span class="station-name">{st.station_name}</span>
                  <span class="station-pad" title={t('outfitting-col-pad')}>{padShort(st.max_pad)}</span>
                  {#if st.discount_tag === 'lyr_15'}
                    <span class="badge badge-green" title={t('outfitting-badge-lyr-tooltip')}>
                      {t('outfitting-badge-lyr')}
                    </span>
                  {:else if st.discount_tag === 'jameson_10'}
                    <span class="badge badge-cyan" title={t('outfitting-badge-elite-tooltip')}>
                      {t('outfitting-badge-elite')}
                    </span>
                  {:else if st.discount_tag === 'brestla_plus20'}
                    <span class="badge badge-amber" title={t('outfitting-badge-brestla-tooltip')}>
                      {t('outfitting-badge-brestla')}
                    </span>
                  {/if}
                  {#if st.is_stronghold_carrier}
                    <span
                      class="badge badge-purple"
                      title={t('outfitting-badge-stronghold-tooltip', {
                        faction: st.controlling_power ?? pledgedPower ?? '',
                      })}
                    >
                      {t('outfitting-badge-stronghold')}
                    </span>
                  {:else if st.is_carrier}
                    <span class="badge badge-orange" title={t('outfitting-badge-carrier-tooltip')}>
                      {t('outfitting-badge-carrier')}
                    </span>
                  {/if}
                  {#if st.is_planetary && !st.is_stronghold_carrier}
                    <span class="badge badge-amber" title={t('outfitting-badge-planetary-tooltip')}>
                      {t('outfitting-badge-planetary')}
                    </span>
                  {/if}
                  {#if st.permit_key}
                    <span class="badge badge-amber" title={t(st.permit_key)}>
                      {t('outfitting-badge-permit')}
                    </span>
                  {/if}
                </div>
                <div class="station-sub-line">
                  <span class="system-name">{st.system_name}</span>
                  <span class="dot-sep">·</span>
                  <span>{ly(st.distance_ly)}</span>
                  <span class="dot-sep">·</span>
                  <span>{formatLs(st.distance_to_arrival_ls)}</span>
                  {#if st.updated_at}
                    <span class="dot-sep">·</span>
                    <span class="age-tag">{formatUpdatedAge(st.updated_at)}</span>
                  {/if}
                </div>
              </div>

              <div class="station-price-actions">
                <div class="price-block">
                  <span class="total-price">
                    {st.total_price_cr != null
                      ? credits(st.total_price_cr)
                      : t('outfitting-price-unknown')}
                  </span>
                  {#if afford}
                    <span class="badge {afford.cls}" title={afford.tooltip}>{afford.label}</span>
                  {/if}
                </div>

                <div class="row-actions">
                  {#if st.item_prices.length > 0}
                    <button
                      type="button"
                      class="hud-btn btn-sm"
                      data-testid="outfitting-breakdown-btn-{idx}"
                      aria-expanded={Boolean(expandedRows[idx])}
                      onclick={() => toggleBreakdown(idx)}
                    >
                      {t('outfitting-breakdown-toggle', { count: st.item_prices.length })}
                    </button>
                  {/if}
                  <button
                    type="button"
                    class="hud-btn btn-sm"
                    data-testid="outfitting-copy-btn-{idx}"
                    title={t('outfitting-copy-tooltip', { system: st.system_name })}
                    onclick={() => copySystem(st.system_name, copyId)}
                  >
                    {copiedKey === copyId ? t('outfitting-copied-btn') : t('outfitting-copy-btn')}
                  </button>
                  <button
                    type="button"
                    class="hud-btn btn-sm"
                    data-testid="outfitting-route-btn-{idx}"
                    title={t('outfitting-route-tooltip', { system: st.system_name })}
                    onclick={() => plotRoute(st.system_name)}
                  >
                    {t('outfitting-route-btn')}
                  </button>
                </div>
              </div>
            </div>

            {#if expandedRows[idx] && st.item_prices.length > 0}
              <div class="itemized-breakdown" data-testid="outfitting-breakdown-{idx}">
                {#each st.item_prices as item, pIdx (`${item.label}-${pIdx}`)}
                  <div class="breakdown-row">
                    <span class="breakdown-label">{item.label}</span>
                    <span class="breakdown-val">
                      {item.price_cr != null ? credits(item.price_cr) : t('outfitting-price-unknown')}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else if status === 'fallback' && fallbackRows.length > 0}
      <div class="fallback-card" data-testid="outfitting-fallback-card">
        <div class="fallback-header">
          <strong>{t('outfitting-fallback-title')}</strong>
          <p>{t('outfitting-fallback-desc')}</p>
        </div>
        <div class="fallback-list">
          {#each fallbackRows as fb, fIdx (`${fb.item_label}-${fIdx}`)}
            {@const fbCopyId = `fb-${fIdx}`}
            <div class="fallback-row">
              <div class="fallback-item-name">{fb.item_label}</div>
              {#if fb.nearest_station}
                {@const st = fb.nearest_station}
                <div class="fallback-station-info">
                  <span class="station-name">{st.station_name}</span>
                  <span class="system-name">{st.system_name}</span>
                  <span class="fallback-dist">{decimal(st.distance_ly, 1)} ly</span>
                  {#if st.total_price_cr != null}
                    <span class="fallback-price">{credits(st.total_price_cr)}</span>
                  {/if}
                  <button
                    type="button"
                    class="hud-btn btn-sm"
                    title={t('outfitting-copy-tooltip', { system: st.system_name })}
                    onclick={() => copySystem(st.system_name, fbCopyId)}
                  >
                    {copiedKey === fbCopyId ? t('outfitting-copied-btn') : t('outfitting-copy-btn')}
                  </button>
                  <button
                    type="button"
                    class="hud-btn btn-sm"
                    title={t('outfitting-route-tooltip', { system: st.system_name })}
                    onclick={() => plotRoute(st.system_name)}
                  >
                    {t('outfitting-route-btn')}
                  </button>
                </div>
              {:else}
                <span class="badge badge-danger">{t('outfitting-fallback-no-stock')}</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</Panel>

<style>
  .outfitting-root {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .search-form {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .top-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 0.65rem;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .field-label {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--hud-muted);
  }

  .origin-input-row {
    display: flex;
    align-items: stretch;
    gap: 0.4rem;
  }

  .autocomplete-wrap {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .hud-input,
  .hud-select {
    width: 100%;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 3px;
    color: var(--hud-text);
    padding: 0.36rem 0.55rem;
    font-size: 0.82rem;
    min-width: 0;
  }

  .hud-input:focus,
  .hud-select:focus {
    outline: none;
    border-color: var(--hud-primary);
  }

  .use-current-btn {
    flex-shrink: 0;
  }

  .suggestions-list {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    z-index: 30;
    margin: 0;
    padding: 0.2rem 0;
    list-style: none;
    background: var(--hud-surface-alt);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
    max-height: 190px;
    overflow-y: auto;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.65);
  }

  .suggestion-item {
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--hud-text);
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
  }

  .suggestion-item:hover {
    background: var(--hud-primary-dim);
    color: var(--hud-text);
  }

  /* Constructor de componentes */
  .module-builder-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.55rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .builder-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .cat-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .cat-pill {
    padding: 0.14rem 0.48rem;
    font-size: 0.7rem;
    font-weight: 600;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
  }

  .cat-pill.active {
    background: rgba(255, 113, 0, 0.16);
    border-color: var(--hud-primary);
    color: var(--hud-primary);
  }

  .builder-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }

  .filter-input {
    flex: 1 1 170px;
  }

  .module-select {
    flex: 2 1 210px;
  }

  .compact-spec {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.74rem;
    color: var(--hud-muted);
    flex-shrink: 0;
  }

  .spec-select {
    width: 68px;
    padding: 0.34rem 0.35rem;
  }

  .mount-select {
    width: 120px;
    padding: 0.34rem 0.4rem;
  }

  .add-module-btn {
    flex-shrink: 0;
  }

  /* Cesta de parada única */
  .basket-box {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    background: rgba(0, 0, 0, 0.22);
    border: 1px dashed var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.5rem 0.7rem;
  }

  .basket-empty {
    font-size: 0.78rem;
    color: var(--hud-muted);
  }

  .basket-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .basket-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.38rem;
    background: rgba(255, 113, 0, 0.12);
    border: 1px solid rgba(255, 113, 0, 0.38);
    color: var(--hud-text);
    padding: 0.2rem 0.48rem;
    border-radius: 3px;
    font-size: 0.77rem;
    font-family: var(--font-mono);
  }

  .basket-pill.ship-pill {
    background: rgba(6, 182, 212, 0.14);
    border-color: rgba(6, 182, 212, 0.45);
  }

  .pill-kind {
    font-size: 0.66rem;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--hud-cyan);
  }

  .pill-remove {
    padding: 0 0.25rem;
    border: none;
    background: transparent;
    color: var(--hud-muted);
    font-size: 0.92rem;
    line-height: 1;
  }

  .pill-remove:hover {
    background: rgba(239, 68, 68, 0.2);
    color: var(--hud-danger);
  }

  /* Barra de filtros e interruptores Glass HUD */
  .filters-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .pad-selector {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .pad-label {
    font-size: 0.74rem;
    color: var(--hud-muted);
    margin-right: 0.15rem;
  }

  .pad-btn {
    padding: 0.2rem 0.48rem;
    font-size: 0.74rem;
    font-family: var(--font-mono);
    font-weight: 700;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    flex-shrink: 0;
  }

  .pad-btn.active {
    background: rgba(255, 113, 0, 0.18);
    border-color: var(--hud-primary);
    color: var(--hud-primary);
  }

  .switches-wrap {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }

  .hud-switch-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.42rem;
    padding: 0.24rem 0.55rem;
    border-radius: 4px;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    font-size: 0.75rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .hud-switch-pill.on {
    border-color: rgba(255, 113, 0, 0.55);
    color: var(--hud-text);
    background: rgba(255, 113, 0, 0.11);
  }

  .hud-switch-pill.stronghold-switch.on {
    border-color: rgba(168, 85, 247, 0.65);
    background: rgba(168, 85, 247, 0.14);
  }

  .hud-switch-track {
    width: 26px;
    height: 14px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.14);
    position: relative;
    flex-shrink: 0;
    transition: background-color 150ms ease;
  }

  .hud-switch-pill.on .hud-switch-track {
    background: var(--hud-primary);
  }

  .hud-switch-pill.stronghold-switch.on .hud-switch-track {
    background: var(--hud-purple);
  }

  .hud-switch-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #fff;
    transition: transform 150ms ease;
  }

  .hud-switch-pill.on .hud-switch-thumb {
    transform: translateX(12px);
  }

  /* Fila dedicada de acciones */
  .form-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .primary-action-btn {
    flex: 2 1 180px;
    justify-content: center;
    background: rgba(255, 113, 0, 0.16);
    border-color: var(--hud-primary);
    color: var(--hud-primary);
    font-weight: 700;
  }

  .secondary-action-btn {
    flex: 1 1 110px;
    justify-content: center;
    color: var(--hud-muted);
    border-color: var(--hud-border-subtle);
  }

  /* Avisos y estados */
  .permit-notice {
    font-size: 0.76rem;
    color: var(--hud-warning);
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.35);
    border-radius: 4px;
    padding: 0.38rem 0.65rem;
  }

  .status-banner {
    font-size: 0.81rem;
    color: var(--hud-muted);
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.6rem 0.8rem;
  }

  .status-banner.warn {
    color: var(--hud-warning);
    border-color: rgba(245, 158, 11, 0.4);
  }

  .status-banner.danger {
    color: var(--hud-danger);
    border-color: rgba(239, 68, 68, 0.4);
  }

  /* Lista adaptativa sin scroll fantasma (AGENTS.md §11.2) */
  .results-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .results-summary {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--hud-muted);
  }

  .stations-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    overflow-y: hidden;
  }

  .stations-list.scrollable {
    max-height: 430px;
    overflow-y: auto;
    padding-right: 0.25rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 113, 0, 0.45) rgba(0, 0, 0, 0.25);
  }

  .station-card {
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.55rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .station-card.highlight {
    border-left: 3px solid var(--hud-primary);
  }

  .station-main-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
  }

  .station-identity {
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
    min-width: 220px;
    flex: 1 1 260px;
  }

  .station-title-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
  }

  .station-name {
    font-weight: 700;
    color: var(--hud-text);
    font-size: 0.88rem;
  }

  .station-pad {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 18px;
    padding: 0 0.3rem;
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-text);
  }

  .station-sub-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.78rem;
    color: var(--hud-muted);
    font-family: var(--font-mono);
  }

  .system-name {
    color: var(--hud-cyan);
    font-weight: 600;
  }

  .dot-sep {
    opacity: 0.5;
  }

  .age-tag {
    font-size: 0.73rem;
  }

  .station-price-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 0.55rem;
  }

  .price-block {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .total-price {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 0.9rem;
    color: #fbbf24;
  }

  .row-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
  }

  .itemized-breakdown {
    border-top: 1px solid var(--hud-border-subtle);
    padding-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
  }

  .breakdown-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.77rem;
    font-family: var(--font-mono);
    color: var(--hud-muted);
  }

  .breakdown-val {
    color: var(--hud-text);
  }

  /* Tarjeta pedagógica de respaldo por pieza */
  .fallback-card {
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.4);
    border-radius: 5px;
    padding: 0.65rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .fallback-header strong {
    color: var(--hud-warning);
    font-size: 0.84rem;
  }

  .fallback-header p {
    margin: 0.2rem 0 0;
    font-size: 0.78rem;
    color: var(--hud-muted);
  }

  .fallback-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .fallback-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    padding: 0.45rem 0.65rem;
  }

  .fallback-item-name {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    font-weight: 700;
    color: var(--hud-text);
  }

  .fallback-station-info {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.42rem;
    font-size: 0.78rem;
  }

  .fallback-dist {
    font-family: var(--font-mono);
    color: var(--hud-muted);
  }

  .fallback-price {
    font-family: var(--font-mono);
    font-weight: 700;
    color: #fbbf24;
  }
</style>
