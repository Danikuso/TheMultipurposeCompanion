<script lang="ts">
  import { onMount } from 'svelte';
  import { getLocale, setLocale, t } from '../lib/i18n';
  import { applyTheme, type HudThemeInput } from '../lib/theme';
  import { loadPrefs, prefs, setAutoCompanion } from '../lib/prefs.svelte';
  import { topic } from '../lib/ws.svelte';
  import Panel from '../shell/Panel.svelte';
  import { diag } from '../lib/diag.svelte';
  import Pairing from './Pairing.svelte';
  import { integer } from '../lib/format';

  const DEFAULT_MINERALS = ['platinum', 'painite', 'lowtemperaturediamond', 'opal', 'alexandrite',
    'grandidierite', 'monazite', 'musgravite', 'rhodplumsite', 'serendibite', 'benitoite'];

  const MAX_KEYS = 30;
  const MAX_KEY_LEN = 40;
  const MAX_THRESHOLD = 1_000_000_000;
  const DEFAULT_ROLLS = [1, 1, 1, 1, 4];
  const validRolls = (value: unknown): value is number[] => Array.isArray(value)
    && value.length === 5 && value.every((n) => Number.isInteger(n) && n >= 1 && n <= 10);

  interface ConfigDto {
    language?: string;
    auto_copy_next_system?: boolean;
    auto_companion?: boolean;
    alerts_enabled?: boolean;
    online_services?: boolean;
    map_value_threshold?: number;
    default_neutron_efficiency?: number;
    theme_mode?: string;
    mining_interest?: string[];
    mining_threshold_cr?: number;
    sell_search_radius_ly?: number;
    eddn_radius_ly?: number;
    tourism_reference_cr_ly?: number;
    engineering_rolls?: number[];
    server?: {
      port?: number;
      lan?: boolean;
    };
  }

  let language = $state(getLocale());
  let themeMode = $state<'hud' | 'classic' | 'contrast'>('hud');
  let autoCopy = $state(true);
  let onlineServices = $state(true);
  let mapThreshold = $state<number | undefined>(500000);
  let plannerEfficiency = $state<number | undefined>(60);
  let lanEnabled = $state(false);
  let miningInterest = $state<string[]>([...DEFAULT_MINERALS]);
  let eddnRadius = $state<number | undefined>(200);
  let tourismReference = $state<number | undefined>(100000);
  let miningThreshold = $state<number | undefined>(1000000);
  let sellRadius = $state<number | undefined>(100);
  let extraMineral = $state('');
  let engineeringRolls = $state<(number | undefined)[]>([...DEFAULT_ROLLS]);
  let rollsInvalid = $state(false);
  let rollsEdits = 0;
  let draftRevision = 0;
  let lastSubmitted: ConfigDto | undefined;
  const knownMinerals = $derived([...DEFAULT_MINERALS, ...miningInterest.filter((k) => !DEFAULT_MINERALS.includes(k))]);
  let fullConfig = $state<ConfigDto>({});
  let saveError = $state(false);
  let validationError = $state(false);
  let saveQueue = Promise.resolve();
  let saveRevision = 0;

  const CLASSIC_THEME: HudThemeInput = {
    primary: '#ff7100',
    secondary: '#00a2ff',
    background: '#0b0c10',
    danger: '#ff2b2b',
  };

  const CONTRAST_THEME: HudThemeInput = {
    primary: '#FFFFFF',
    secondary: '#00FFFF',
    background: '#000000',
    danger: '#FF4444',
  };

  function applySelectedTheme(mode: 'hud' | 'classic' | 'contrast') {
    if (mode === 'contrast') {
      applyTheme(CONTRAST_THEME);
    } else if (mode === 'classic') {
      applyTheme(CLASSIC_THEME);
    } else {
      const liveHud = topic<HudThemeInput>('core.theme') ?? CLASSIC_THEME;
      applyTheme(liveHud);
    }
  }

  onMount(() => {
    lastSubmitted = currentConfig();
    saveQueue = readConfig();
  });

  async function readConfig() {
    try {
      const res = await fetch('/api/config');
      diag('ui.fetch.result', { kind: 'config-get', ok: res.ok, status: res.status });
      if (res.ok) {
        const cfg = (await res.json()) as ConfigDto;
        fullConfig = cfg;
        // Sin envíos pendientes, comparar la primera edición con lo leído del
        // servidor. Los envíos hechos antes del GET conservan su propia referencia.
        if (saveRevision === 0) lastSubmitted = undefined;
        loadPrefs(cfg);
        onlineServices = cfg.online_services ?? true;
        mapThreshold = cfg.map_value_threshold ?? 500000;
        plannerEfficiency = cfg.default_neutron_efficiency ?? 60;
        miningInterest = cfg.mining_interest ?? [...DEFAULT_MINERALS];
        miningThreshold = cfg.mining_threshold_cr ?? 1000000;
        sellRadius = cfg.sell_search_radius_ly ?? 100;
        eddnRadius = cfg.eddn_radius_ly ?? 200;
        tourismReference = cfg.tourism_reference_cr_ly ?? 100000;
        if (rollsEdits === 0) engineeringRolls = [...(validRolls(cfg.engineering_rolls) ? cfg.engineering_rolls : DEFAULT_ROLLS)];
        if (cfg.language) {
          language = cfg.language;
          setLocale(cfg.language);
        }
        if (typeof cfg.auto_copy_next_system === 'boolean') {
          autoCopy = cfg.auto_copy_next_system;
        }
        if (cfg.server && typeof cfg.server.lan === 'boolean') {
          lanEnabled = cfg.server.lan;
        }
        if (cfg.theme_mode === 'classic' || cfg.theme_mode === 'contrast' || cfg.theme_mode === 'hud') {
          themeMode = cfg.theme_mode;
          applySelectedTheme(themeMode);
        }
      }
    } catch {
      // Offline or unconfigured
      diag('ui.fetch.result', { kind: 'config-get', ok: false, status: 0 });
    }
  }

  function applyConfirmedPreferences() {
    language = fullConfig.language ?? getLocale();
    themeMode = fullConfig.theme_mode === 'classic' || fullConfig.theme_mode === 'contrast' ? fullConfig.theme_mode : 'hud';
    setLocale(language);
    applySelectedTheme(themeMode);
  }

  function restoreSavedConfig() {
    autoCopy = fullConfig.auto_copy_next_system ?? true;
    onlineServices = fullConfig.online_services ?? true;
    mapThreshold = fullConfig.map_value_threshold ?? 500000;
    plannerEfficiency = fullConfig.default_neutron_efficiency ?? 60;
    lanEnabled = fullConfig.server?.lan ?? false;
    miningInterest = fullConfig.mining_interest ?? [...DEFAULT_MINERALS];
    miningThreshold = fullConfig.mining_threshold_cr ?? 1000000;
    sellRadius = fullConfig.sell_search_radius_ly ?? 100;
    eddnRadius = fullConfig.eddn_radius_ly ?? 200;
    tourismReference = fullConfig.tourism_reference_cr_ly ?? 100000;
    engineeringRolls = [...(validRolls(fullConfig.engineering_rolls) ? fullConfig.engineering_rolls : DEFAULT_ROLLS)];
  }

  function mineralLabel(key: string): string {
    const label = t(`mineral-${key}`);
    return !label || label === `mineral-${key}` ? key : label;
  }

  function toggleMineral(key: string, checked: boolean) {
    miningInterest = checked
      ? (miningInterest.includes(key) ? miningInterest : [...miningInterest, key])
      : miningInterest.filter((k) => k !== key);
    saveConfig();
  }

  function addMineral() {
    const key = extraMineral.toLowerCase().replace(/\s+/g, '');
    extraMineral = '';
    if (!key) return;
    if (key.length > MAX_KEY_LEN || (!miningInterest.includes(key) && miningInterest.length >= MAX_KEYS)) {
      validationError = true;
      return;
    }
    toggleMineral(key, true);
  }

  function currentConfig(): ConfigDto {
    return {
      language,
      auto_copy_next_system: autoCopy,
      auto_companion: prefs.autoCompanion,
      online_services: onlineServices,
      map_value_threshold: mapThreshold,
      default_neutron_efficiency: plannerEfficiency,
      theme_mode: themeMode,
      mining_interest: [...miningInterest],
      mining_threshold_cr: miningThreshold,
      sell_search_radius_ly: sellRadius,
      eddn_radius_ly: eddnRadius,
      tourism_reference_cr_ly: tourismReference,
      engineering_rolls: [...engineeringRolls] as number[],
      server: {
        lan: lanEnabled,
      },
    };
  }

  function restoreConfirmedRolls(submitted: number[]) {
    const saved = validRolls(fullConfig.engineering_rolls) ? fullConfig.engineering_rolls : DEFAULT_ROLLS;
    // Revierte solo los grados que no se han vuelto a editar durante el PUT.
    engineeringRolls = engineeringRolls.map((value, index) => value === submitted[index] ? saved[index] : value);
  }

  function saveConfig() {
    validationError = false;
    rollsInvalid = !validRolls(engineeringRolls);
    if (rollsInvalid) return;
    if (mapThreshold === undefined || !Number.isSafeInteger(mapThreshold) || mapThreshold < 0
      || plannerEfficiency === undefined || !Number.isInteger(plannerEfficiency)
      || plannerEfficiency < 0 || plannerEfficiency > 100
      || miningThreshold === undefined || !Number.isSafeInteger(miningThreshold) || miningThreshold < 0 || miningThreshold > MAX_THRESHOLD
      || miningInterest.length > MAX_KEYS || miningInterest.some((k) => k.length > MAX_KEY_LEN)
      || sellRadius === undefined || !Number.isInteger(sellRadius) || sellRadius < 10 || sellRadius > 500
      || eddnRadius === undefined || !Number.isInteger(eddnRadius) || eddnRadius < 20 || eddnRadius > 500
      || tourismReference === undefined || !Number.isInteger(tourismReference) || tourismReference < 1000 || tourismReference > 10_000_000) { validationError = true; return; }
    saveError = false;
    const revision = ++saveRevision;
    const draft = draftRevision;
    const updated = currentConfig();
    // Solo se encolan los campos editados desde el envío anterior. Un error en
    // tiradas no debe reaplicarse al guardar después otro ajuste.
    const patch = Object.fromEntries(Object.entries(updated).filter(([key, value]) =>
      JSON.stringify(value) !== JSON.stringify((lastSubmitted ?? fullConfig)[key as keyof ConfigDto]))) as ConfigDto;
    lastSubmitted = updated;
    saveQueue = saveQueue.then(async () => {
      try {
        const res = await fetch('/api/config', {
          method: 'PUT',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ ...fullConfig, ...patch, server: { ...fullConfig.server, ...patch.server } }),
        });
        // Nunca se vuelca la configuración: contiene el token LAN en `server`.
        diag('ui.fetch.result', { kind: 'config-put', ok: res.ok, status: res.status, lan: lanEnabled });
        if (!res.ok) throw new Error('config-save-failed');
        fullConfig = (await res.json()) as ConfigDto;
        if (revision === saveRevision) {
          applyConfirmedPreferences();
          if (draft === draftRevision) restoreSavedConfig();
          else restoreConfirmedRolls(updated.engineering_rolls!);
          lastSubmitted = undefined;
        }
      } catch {
        diag('ui.fetch.result', { kind: 'config-put', ok: false, status: 0, lan: lanEnabled });
        if (revision === saveRevision) {
          applyConfirmedPreferences();
          if (draft === draftRevision) restoreSavedConfig();
          else restoreConfirmedRolls(updated.engineering_rolls!);
          lastSubmitted = undefined;
          saveError = true;
        }
      }
    });
  }
</script>

<Panel titleKey="nav-settings">
  {#if validationError}<p role="alert" data-testid="validation-error">{t('settings-mining-invalid')}</p>{/if}
  {#if saveError}<p role="alert">{t('settings-save-error')}</p>{/if}
  <div class="settings-form" oninput={() => { draftRevision++; }}>
    <label class="field-row">
      <span>{t('settings-language')}</span>
      <select bind:value={language} onchange={saveConfig}>
        <option value="es">Español (es)</option>
        <option value="en">English (en)</option>
        <option value="fr">Français (fr)</option>
        <option value="de">Deutsch (de)</option>
        <option value="pt">Português (pt)</option>
      </select>
    </label>

    <label class="field-row">
      <span>{t('settings-theme')}</span>
      <select bind:value={themeMode} onchange={saveConfig}>
        <option value="hud">{t('theme-hud')}</option>
        <option value="classic">{t('theme-classic')}</option>
        <option value="contrast">#FFFFFF / #000000</option>
      </select>
    </label>

    <label class="field-row">
      <span>{t('settings-auto-copy')}</span>
      <input type="checkbox" bind:checked={autoCopy} onchange={saveConfig} />
    </label>

    <label class="field-row" title={t('auto-companion-tooltip')}>
      <span>{t('auto-companion')}</span>
      <input type="checkbox" role="switch" data-testid="settings-auto-companion" checked={prefs.autoCompanion}
        onchange={(e) => { void setAutoCompanion(e.currentTarget.checked); }} />
    </label>
    <p class="help-text">{t('auto-companion-tooltip')}</p>

    <label class="field-row">
      <span>{t('settings-lan')}</span>
      <input type="checkbox" bind:checked={lanEnabled} onchange={saveConfig} />
    </label>
    <p class="help-text">{t('settings-lan-help')}</p>

    <label class="field-row">
      <span>{t('settings-online-services')}</span>
      <input type="checkbox" bind:checked={onlineServices} onchange={saveConfig} data-testid="online-services"
        aria-describedby="online-services-help" />
    </label>
    <p id="online-services-help" class="help-text">{t('settings-online-services-help')}</p>
    <label class="field-row">
      <span>{t('settings-map-threshold')}</span>
      <input type="number" min="0" step="1" bind:value={mapThreshold} onchange={saveConfig} data-testid="map-threshold" />
    </label>
    <label class="field-row">
      <span>{t('settings-planner-efficiency')}</span>
      <input type="number" min="0" max="100" step="1" bind:value={plannerEfficiency} onchange={saveConfig} data-testid="planner-efficiency" />
    </label>

    <fieldset class="minerals" data-testid="mining-interest">
      <legend>{t('settings-mining-interest')}</legend>
      {#each knownMinerals as key (key)}
        <label class="mineral">
          <input type="checkbox" data-mineral={key} checked={miningInterest.includes(key)}
            onchange={(e) => toggleMineral(key, e.currentTarget.checked)} />
          <span>{mineralLabel(key)}</span>
        </label>
      {/each}
      <div class="field-row">
        <input type="text" bind:value={extraMineral} data-testid="mining-extra" aria-label={t('settings-mining-add')}
          onkeydown={(e) => { if (e.key === 'Enter') addMineral(); }} />
        <button type="button" data-testid="mining-add" onclick={addMineral}>{t("settings-mining-add-button")}</button>
      </div>
    </fieldset>
    <label class="field-row">
      <span>{t('settings-mining-threshold')}</span>
      <input type="number" min="0" max="1000000000" step="1" bind:value={miningThreshold} onchange={saveConfig} data-testid="mining-threshold" />
    </label>
    <label class="field-row">
      <span>{t('settings-sell-radius')}</span>
      <input type="number" min="10" max="500" step="1" bind:value={sellRadius} onchange={saveConfig} data-testid="sell-radius" />
    </label>
    <label class="field-row">
      <span>{t('settings-eddn-radius')}</span>
      <input type="number" min="20" max="500" step="1" bind:value={eddnRadius} onchange={saveConfig} data-testid="eddn-radius" />
    </label>
    <label class="field-row">
      <span>{t('settings-tourism-reference')}</span>
      <input type="number" min="1000" max="10000000" step="1" bind:value={tourismReference} onchange={saveConfig} data-testid="tourism-reference" />
    </label>

    <fieldset class="minerals engineering-rolls" data-testid="engineering-rolls">
      <legend>{t('settings-engineering-rolls')}</legend>
      {#each [1, 2, 3, 4, 5] as grade}
        <label class="field-row" for={`engineering-rolls-${grade}`}>
          <span>{t('engineering-grade', { grade: integer(grade) })}</span>
          <input id={`engineering-rolls-${grade}`} data-testid={`engineering-rolls-${grade}`}
            type="number" min="1" max="10" step="1" bind:value={engineeringRolls[grade - 1]}
            aria-invalid={rollsInvalid} aria-describedby={rollsInvalid ? 'engineering-rolls-error' : undefined}
            oninput={() => { rollsEdits++; }} onchange={saveConfig} />
        </label>
      {/each}
      {#if rollsInvalid}<p id="engineering-rolls-error" role="alert" data-testid="engineering-rolls-error">{t('settings-engineering-rolls-invalid')}</p>{/if}
    </fieldset>

    <Pairing />
  </div>
</Panel>

<style>
  .settings-form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    max-width: 520px;
  }

  .field-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  select {
    background: var(--hud-bg);
    border: 1px solid var(--hud-border);
    padding: 0.3rem 0.5rem;
    border-radius: 3px;
  }

  .minerals { border: 1px solid var(--hud-border); display: flex; flex-direction: column; gap: 0.3rem; }
  .mineral { display: flex; gap: 0.5rem; align-items: center; }

  .engineering-rolls input {
    background: var(--hud-bg);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 3px;
    padding: 0.3rem 0.5rem;
  }

  .help-text {
    margin: 0;
    font-size: 0.8rem;
    color: var(--hud-muted);
  }
</style>
