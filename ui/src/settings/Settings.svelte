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
  import { voiceAssistant } from '../lib/voiceAssistant.svelte';

  const DEFAULT_MINERALS = ['platinum', 'painite', 'lowtemperaturediamond', 'opal', 'alexandrite',
    'grandidierite', 'monazite', 'musgravite', 'rhodplumsite', 'serendibite', 'benitoite'];

  const MAX_KEYS = 30;
  const MAX_KEY_LEN = 40;
  const MAX_THRESHOLD = 1_000_000_000;
  const DEFAULT_ROLLS = [1, 1, 1, 1, 4];
  const validRolls = (value: unknown): value is number[] => Array.isArray(value)
    && value.length === 5 && value.every((n) => Number.isInteger(n) && n >= 1 && n <= 10);

  interface TtsSettingsDto {
    enabled?: boolean;
    engine?: 'native' | 'kokoro';
    voice?: string;
    volume?: number;
    rate?: number;
    channels?: {
      combat?: boolean;
      navigation?: boolean;
      exploration?: boolean;
      mining?: boolean;
      carrier?: boolean;
    };
  }

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
    tts?: TtsSettingsDto;
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

  let ttsEnabled = $state(false);
  let ttsEngine = $state<'native' | 'kokoro'>('native');
  let ttsVoice = $state('');
  let ttsVolume = $state(80);
  let ttsRate = $state(100);
  let ttsChannels = $state({
    combat: true,
    navigation: true,
    exploration: true,
    mining: true,
    carrier: true,
  });
  let ttsVoicesList = $state<Array<{ id: string; name: string; lang: string; gender?: string }>>([]);
  let ttsTesting = $state(false);

  async function loadTtsVoices(engine: 'native' | 'kokoro') {
    try {
      const res = await fetch(`/api/tts/voices?engine=${engine}`);
      if (res.ok) {
        const data = await res.json();
        ttsVoicesList = data.voices ?? [];
      }
    } catch {
      // offline / mock
    }
  }

  function onEngineChange() {
    ttsVoice = '';
    void loadTtsVoices(ttsEngine);
    saveConfig();
  }

  async function testTtsVoice() {
    ttsTesting = true;
    try {
      await fetch('/api/tts/test', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          engine: ttsEngine,
          voice: ttsVoice || undefined,
          volume: ttsVolume,
          rate: ttsRate,
        }),
      });
    } catch {
      // ignore
    } finally {
      setTimeout(() => {
        ttsTesting = false;
      }, 2000);
    }
  }
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
    void voiceAssistant.fetchVoiceAttackStatus();
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
        if (cfg.tts) {
          ttsEnabled = cfg.tts.enabled ?? false;
          ttsEngine = cfg.tts.engine ?? 'native';
          ttsVoice = cfg.tts.voice ?? '';
          ttsVolume = cfg.tts.volume ?? 80;
          ttsRate = cfg.tts.rate ?? 100;
          if (cfg.tts.channels) {
            ttsChannels = {
              combat: cfg.tts.channels.combat ?? true,
              navigation: cfg.tts.channels.navigation ?? true,
              exploration: cfg.tts.channels.exploration ?? true,
              mining: cfg.tts.channels.mining ?? true,
              carrier: cfg.tts.channels.carrier ?? true,
            };
          }
        }
        void loadTtsVoices(ttsEngine);
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
    if (fullConfig.tts) {
      ttsEnabled = fullConfig.tts.enabled ?? false;
      ttsEngine = fullConfig.tts.engine ?? 'native';
      ttsVoice = fullConfig.tts.voice ?? '';
      ttsVolume = fullConfig.tts.volume ?? 80;
      ttsRate = fullConfig.tts.rate ?? 100;
      if (fullConfig.tts.channels) {
        ttsChannels = {
          combat: fullConfig.tts.channels.combat ?? true,
          navigation: fullConfig.tts.channels.navigation ?? true,
          exploration: fullConfig.tts.channels.exploration ?? true,
          mining: fullConfig.tts.channels.mining ?? true,
          carrier: fullConfig.tts.channels.carrier ?? true,
        };
      }
    }
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
      tts: {
        enabled: ttsEnabled,
        engine: ttsEngine,
        voice: ttsVoice || undefined,
        volume: ttsVolume,
        rate: ttsRate,
        channels: { ...ttsChannels },
      },
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

    <fieldset class="minerals tts-section" data-testid="tts-settings">
      <legend>{t('settings-tts-title')}</legend>
      <p class="help-text">{t('settings-tts-description')}</p>

      <label class="field-row">
        <span>{t('settings-tts-enabled')}</span>
        <input
          type="checkbox"
          role="switch"
          data-testid="tts-enabled"
          bind:checked={ttsEnabled}
          onchange={saveConfig}
        />
      </label>

      {#if ttsEnabled}
        <label class="field-row">
          <span>{t('settings-tts-engine')}</span>
          <select
            data-testid="tts-engine"
            bind:value={ttsEngine}
            onchange={onEngineChange}
          >
            <option value="native">{t('settings-tts-engine-native')}</option>
            <option value="kokoro">{t('settings-tts-engine-kokoro')}</option>
          </select>
        </label>

        <label class="field-row">
          <span>{t('settings-tts-voice')}</span>
          <select
            data-testid="tts-voice"
            bind:value={ttsVoice}
            onchange={saveConfig}
          >
            <option value="">(Predeterminada)</option>
            {#each ttsVoicesList as v (v.id)}
              <option value={v.id}>{v.name} ({v.lang})</option>
            {/each}
          </select>
        </label>

        <label class="field-row">
          <span>{t('settings-tts-volume')} ({ttsVolume}%)</span>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            data-testid="tts-volume"
            bind:value={ttsVolume}
            onchange={saveConfig}
          />
        </label>

        <label class="field-row">
          <span>{t('settings-tts-rate')} ({ttsRate}%)</span>
          <input
            type="range"
            min="80"
            max="150"
            step="5"
            data-testid="tts-rate"
            bind:value={ttsRate}
            onchange={saveConfig}
          />
        </label>

        <div class="tts-channels">
          <span class="channels-title">{t('settings-tts-channels')}</span>
          <div class="channels-grid">
            <label class="channel-item">
              <input
                type="checkbox"
                data-testid="tts-channel-combat"
                bind:checked={ttsChannels.combat}
                onchange={saveConfig}
              />
              <span>{t('settings-tts-channel-combat')}</span>
            </label>
            <label class="channel-item">
              <input
                type="checkbox"
                data-testid="tts-channel-navigation"
                bind:checked={ttsChannels.navigation}
                onchange={saveConfig}
              />
              <span>{t('settings-tts-channel-navigation')}</span>
            </label>
            <label class="channel-item">
              <input
                type="checkbox"
                data-testid="tts-channel-exploration"
                bind:checked={ttsChannels.exploration}
                onchange={saveConfig}
              />
              <span>{t('settings-tts-channel-exploration')}</span>
            </label>
            <label class="channel-item">
              <input
                type="checkbox"
                data-testid="tts-channel-mining"
                bind:checked={ttsChannels.mining}
                onchange={saveConfig}
              />
              <span>{t('settings-tts-channel-mining')}</span>
            </label>
            <label class="channel-item">
              <input
                type="checkbox"
                data-testid="tts-channel-carrier"
                bind:checked={ttsChannels.carrier}
                onchange={saveConfig}
              />
              <span>{t('settings-tts-channel-carrier')}</span>
            </label>
          </div>
        </div>

        <div class="test-row">
          <button
            type="button"
            class="btn-test-voice"
            data-testid="tts-test-btn"
            disabled={ttsTesting}
            onclick={testTtsVoice}
          >
            {ttsTesting ? t('settings-tts-test-playing') : t('settings-tts-test-button')}
          </button>
        </div>
      {/if}
    </fieldset>

    <!-- Asistencia por Voz e Integración («TMC Glass HUD») -->
    <fieldset class="tts-section voice-assistant-section">
      <legend>🎙️ {t('voice-assistant-title')}</legend>
      <div class="field-row">
        <label for="voice-assistant-toggle">
          <span>{t('voice-assistant-enable')}</span>
          <small class="field-hint">{t('voice-assistant-enable-desc')}</small>
        </label>
        <input
          id="voice-assistant-toggle"
          type="checkbox"
          data-testid="voice-assistant-toggle"
          checked={voiceAssistant.enabled}
          onchange={(e) => voiceAssistant.setEnabled(e.currentTarget.checked)}
        />
      </div>

      {#if voiceAssistant.enabled}
        <div class="field-row">
          <label for="voice-engine-select">
            <span>{t('voice-assistant-engine')}</span>
          </label>
          <select
            id="voice-engine-select"
            data-testid="voice-engine-select"
            value={voiceAssistant.sttEngine}
            onchange={(e) => voiceAssistant.setSttEngine(e.currentTarget.value as 'whisper' | 'native')}
          >
            <option value="whisper">{t('voice-assistant-engine-whisper')}</option>
            <option value="native">{t('voice-assistant-engine-native')}</option>
          </select>
        </div>

        {#if voiceAssistant.sttEngine === 'whisper'}
          <div class="whisper-status-card">
            <div class="whisper-status-header">
              <span class="whisper-title">🧠 Whisper AI Local</span>
              <span
                class="badge"
                class:badge-success={voiceAssistant.whisperStatus === 'ready'}
                class:badge-warning={voiceAssistant.whisperStatus === 'loading'}
                class:badge-muted={voiceAssistant.whisperStatus === 'unloaded'}
                data-testid="whisper-status-badge"
              >
                {#if voiceAssistant.whisperStatus === 'ready'}
                  {t('voice-assistant-whisper-ready')}
                {:else if voiceAssistant.whisperStatus === 'loading'}
                  {t('voice-assistant-whisper-loading', { pct: voiceAssistant.whisperProgress })}
                {:else}
                  Offline (~39MB)
                {/if}
              </span>
            </div>
            <p class="whisper-desc">{t('voice-assistant-whisper-model-info')}</p>
            {#if voiceAssistant.whisperStatus !== 'ready'}
              <div class="whisper-actions">
                <button
                  type="button"
                  class="btn-preload-whisper"
                  data-testid="preload-whisper-btn"
                  disabled={voiceAssistant.whisperStatus === 'loading'}
                  onclick={() => void voiceAssistant.preloadWhisperModel()}
                >
                  ⚡ {t('voice-assistant-whisper-preload')}
                </button>
              </div>
            {/if}
          </div>
        {/if}

        <div class="field-row">
          <label for="voice-mode-select">
            <span>{t('voice-assistant-mode')}</span>
          </label>
          <select
            id="voice-mode-select"
            data-testid="voice-mode-select"
            value={voiceAssistant.mode}
            onchange={(e) => voiceAssistant.setMode(e.currentTarget.value as 'continuous' | 'ptt')}
          >
            <option value="continuous">{t('voice-assistant-mode-continuous')}</option>
            <option value="ptt">{t('voice-assistant-mode-ptt')}</option>
          </select>
        </div>

        {#if voiceAssistant.lastQuery}
          <div class="voice-feedback-row">
            <span class="feedback-label">{t('voice-assistant-last-query')}:</span>
            <span class="feedback-val font-mono">"{voiceAssistant.lastQuery}"</span>
          </div>
        {/if}
      {/if}

      <!-- Integración con VoiceAttack -->
      <div class="voiceattack-box">
        <div class="voiceattack-header">
          <span class="va-title">⚡ {t('voiceattack-section-title')}</span>
          <span
            class="badge"
            class:badge-success={voiceAssistant.voiceAttackStatus?.installed}
            class:badge-muted={!voiceAssistant.voiceAttackStatus?.installed}
            data-testid="voiceattack-badge"
          >
            {voiceAssistant.voiceAttackStatus?.installed
              ? (voiceAssistant.voiceAttackStatus?.running
                  ? `${t('voiceattack-status-detected')} (${t('voiceattack-status-running')})`
                  : `${t('voiceattack-status-detected')} (${t('voiceattack-status-stopped')})`)
              : t('voiceattack-status-not-detected')}
          </span>
        </div>

        <p class="va-desc">{t('voiceattack-help-desc')}</p>

        <div class="va-actions">
          <a
            href="/api/voice/voiceattack/profile"
            download="TMC-Copiloto.vap"
            class="btn-download-vap"
            data-testid="download-vap-btn"
          >
            📥 {t('voiceattack-download-profile')}
          </a>
        </div>
      </div>
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

  .tts-section {
    border: 1px solid var(--hud-border);
    padding: 0.75rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .tts-section legend {
    font-weight: 600;
    color: var(--hud-primary, #ff7100);
    padding: 0 0.4rem;
  }

  .tts-channels {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-top: 0.3rem;
  }

  .channels-title {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--hud-primary, #ff7100);
  }

  .channels-grid {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding-left: 0.5rem;
  }

  .channel-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .test-row {
    margin-top: 0.5rem;
    display: flex;
    justify-content: flex-end;
  }

  .btn-test-voice {
    background: var(--hud-bg);
    color: var(--hud-primary, #ff7100);
    border: 1px solid var(--hud-border);
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }

  .btn-test-voice:hover:not(:disabled) {
    background: var(--hud-primary, #ff7100);
    color: var(--hud-bg, #000);
  }

  .btn-test-voice:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  .voice-assistant-section {
    border-color: rgba(0, 212, 178, 0.25);
  }

  .voice-assistant-section legend {
    color: var(--hud-cyan, #00d4b2);
  }

  .whisper-status-card {
    padding: 0.6rem 0.75rem;
    background: rgba(0, 212, 178, 0.05);
    border: 1px solid rgba(0, 212, 178, 0.2);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin: 0.25rem 0;
  }

  .whisper-status-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .whisper-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--hud-cyan, #00d4b2);
  }

  .whisper-desc {
    margin: 0;
    font-size: 0.75rem;
    color: var(--hud-muted);
    line-height: 1.35;
  }

  .whisper-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.2rem;
  }

  .btn-preload-whisper {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(0, 212, 178, 0.15);
    color: var(--hud-cyan, #00d4b2);
    border: 1px solid rgba(0, 212, 178, 0.4);
    padding: 0.3rem 0.65rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-preload-whisper:hover:not(:disabled) {
    background: var(--hud-cyan, #00d4b2);
    color: #000;
  }

  .btn-preload-whisper:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .voice-feedback-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: rgba(0, 212, 178, 0.08);
    border: 1px dashed rgba(0, 212, 178, 0.3);
    border-radius: 4px;
    font-size: 0.8rem;
  }

  .feedback-label {
    color: var(--hud-muted);
  }

  .feedback-val {
    color: var(--hud-cyan, #00d4b2);
  }

  .voiceattack-box {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: var(--hud-surface-card, rgba(20, 25, 35, 0.6));
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .voiceattack-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .va-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--hud-text);
  }

  .va-desc {
    margin: 0;
    font-size: 0.75rem;
    color: var(--hud-muted);
    line-height: 1.4;
  }

  .va-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.25rem;
  }

  .btn-download-vap {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(0, 212, 178, 0.12);
    color: var(--hud-cyan, #00d4b2);
    border: 1px solid rgba(0, 212, 178, 0.3);
    padding: 0.35rem 0.75rem;
    border-radius: 4px;
    text-decoration: none;
    font-size: 0.8rem;
    font-weight: 600;
    transition: all 0.15s ease;
  }

  .btn-download-vap:hover {
    background: var(--hud-cyan, #00d4b2);
    color: #000;
  }
</style>
