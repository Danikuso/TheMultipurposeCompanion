<script lang="ts">
  import { onMount } from 'svelte';
  import { getLocale, setLocale, t } from '../lib/i18n';
  import { applyTheme, type HudThemeInput } from '../lib/theme';
  import { loadPrefs, prefs, setAutoCompanion } from '../lib/prefs.svelte';
  import { DEFAULT_PRIORITIES } from '../shell/layouts';
  import { topic } from '../lib/ws.svelte';
  import Panel from '../shell/Panel.svelte';
  import { diag } from '../lib/diag.svelte';
  import Pairing from './Pairing.svelte';
  import { integer } from '../lib/format';
  import { voiceAssistant } from '../lib/voiceAssistant.svelte';
  import AccountsSection from '../modules/accounts/AccountsSection.svelte';

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
    auto_companion_priorities?: string[];
    auto_companion_linger_seconds?: number;
    auto_companion_voice_cues?: boolean;
    auto_companion_sound_cues?: boolean;
    auto_companion_predictive?: boolean;
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
  let autoPriorities = $state<string[]>([...prefs.autoCompanionPriorities]);
  let lingerSeconds = $state<number>(prefs.autoCompanionLingerSeconds);
  let voiceCues = $state<boolean>(prefs.autoCompanionVoiceCues);
  let soundCues = $state<boolean>(prefs.autoCompanionSoundCues);
  let predictive = $state<boolean>(prefs.autoCompanionPredictive);

  function movePriority(index: number, direction: 'up' | 'down') {
    const target = direction === 'up' ? index - 1 : index + 1;
    if (target < 0 || target >= autoPriorities.length) return;
    const list = [...autoPriorities];
    const [moved] = list.splice(index, 1);
    list.splice(target, 0, moved);
    autoPriorities = list;
    prefs.autoCompanionPriorities = [...list];
    saveConfig();
  }

  function resetPriorities() {
    autoPriorities = [...DEFAULT_PRIORITIES];
    prefs.autoCompanionPriorities = [...DEFAULT_PRIORITIES];
    saveConfig();
  }
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
        if (Array.isArray(cfg.auto_companion_priorities)) {
          autoPriorities = [...cfg.auto_companion_priorities];
          prefs.autoCompanionPriorities = [...autoPriorities];
        }
        if (typeof cfg.auto_companion_linger_seconds === 'number') {
          lingerSeconds = cfg.auto_companion_linger_seconds;
          prefs.autoCompanionLingerSeconds = lingerSeconds;
        }
        if (typeof cfg.auto_companion_voice_cues === 'boolean') {
          voiceCues = cfg.auto_companion_voice_cues;
          prefs.autoCompanionVoiceCues = voiceCues;
        }
        if (typeof cfg.auto_companion_sound_cues === 'boolean') {
          soundCues = cfg.auto_companion_sound_cues;
          prefs.autoCompanionSoundCues = soundCues;
        }
        if (typeof cfg.auto_companion_predictive === 'boolean') {
          predictive = cfg.auto_companion_predictive;
          prefs.autoCompanionPredictive = predictive;
        }
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
    if (Array.isArray(fullConfig.auto_companion_priorities)) {
      autoPriorities = [...fullConfig.auto_companion_priorities];
      prefs.autoCompanionPriorities = [...autoPriorities];
    }
    if (typeof fullConfig.auto_companion_linger_seconds === 'number') {
      lingerSeconds = fullConfig.auto_companion_linger_seconds;
      prefs.autoCompanionLingerSeconds = lingerSeconds;
    }
    if (typeof fullConfig.auto_companion_voice_cues === 'boolean') {
      voiceCues = fullConfig.auto_companion_voice_cues;
      prefs.autoCompanionVoiceCues = voiceCues;
    }
    if (typeof fullConfig.auto_companion_sound_cues === 'boolean') {
      soundCues = fullConfig.auto_companion_sound_cues;
      prefs.autoCompanionSoundCues = soundCues;
    }
    if (typeof fullConfig.auto_companion_predictive === 'boolean') {
      predictive = fullConfig.auto_companion_predictive;
      prefs.autoCompanionPredictive = predictive;
    }
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
      auto_companion_priorities: [...autoPriorities],
      auto_companion_linger_seconds: lingerSeconds,
      auto_companion_voice_cues: voiceCues,
      auto_companion_sound_cues: soundCues,
      auto_companion_predictive: predictive,
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

  function openGitHubFeedback() {
    const tmcVersion = '0.1.0';
    const osInfo = typeof navigator !== 'undefined' ? navigator.userAgent : 'Windows';
    const lang = fullConfig.language || 'es';
    const issueTitle = encodeURIComponent('[Feedback / Reporte] ');
    const issueBody = encodeURIComponent(
`### Descripción de la incidencia o propuesta:
<!-- Escribe aquí detalladamente lo que has observado o tu sugerencia -->

---
### Pasos para reproducir (si aplica):
1. 
2. 
3. 

---
### Diagnóstico de entorno:
- **Versión de TMC:** ${tmcVersion}
- **Idioma activo:** ${lang}
- **Plataforma / Agente:** ${osInfo}
`
    );
    const url = `https://github.com/Danikuso/TheMultipurposeCompanion/issues/new?title=${issueTitle}&body=${issueBody}`;
    if (typeof window !== 'undefined') {
      window.open(url, '_blank');
    }
  }
</script>

<Panel titleKey="nav-settings" span="full">
  {#if validationError}<p role="alert" data-testid="validation-error">{t('settings-mining-invalid')}</p>{/if}
  {#if saveError}<p role="alert">{t('settings-save-error')}</p>{/if}
  <div class="settings-form" oninput={() => { draftRevision++; }}>
    <div class="settings-two-panes">
      <div class="settings-main-pane">
        <div class="settings-grid">
      <!-- Columna 1: Configuración General, Minería y Tiradas de Ingeniería -->
      <div class="settings-col">
        <fieldset class="settings-card">
          <legend>⚙️ {t('nav-settings')}</legend>

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
            <input type="checkbox" role="switch" class="hud-switch" bind:checked={autoCopy} onchange={saveConfig} />
          </label>

          <div class="toggle-group">
            <label class="field-row" title={t('auto-companion-tooltip')}>
              <span>{t('auto-companion')}</span>
              <input type="checkbox" role="switch" class="hud-switch" data-testid="settings-auto-companion" checked={prefs.autoCompanion}
                onchange={(e) => { void setAutoCompanion(e.currentTarget.checked); }} />
            </label>
            <p class="help-text">{t('auto-companion-tooltip')}</p>
          </div>

          <div class="toggle-group">
            <label class="field-row">
              <span>{t('settings-lan')}</span>
              <input type="checkbox" role="switch" class="hud-switch" bind:checked={lanEnabled} onchange={saveConfig} />
            </label>
            <p class="help-text">{t('settings-lan-help')}</p>
          </div>

          <div class="toggle-group">
            <label class="field-row">
              <span>{t('settings-online-services')}</span>
              <input type="checkbox" role="switch" class="hud-switch" bind:checked={onlineServices} onchange={saveConfig} data-testid="online-services"
                aria-describedby="online-services-help" />
            </label>
            <p id="online-services-help" class="help-text">{t('settings-online-services-help')}</p>
          </div>

          <label class="field-row">
            <span>{t('settings-map-threshold')}</span>
            <input type="number" min="0" step="1" bind:value={mapThreshold} onchange={saveConfig} data-testid="map-threshold" />
          </label>
          <label class="field-row">
            <span>{t('settings-planner-efficiency')}</span>
            <input type="number" min="0" max="100" step="1" bind:value={plannerEfficiency} onchange={saveConfig} data-testid="planner-efficiency" />
          </label>
        </fieldset>

        <fieldset class="settings-card auto-companion-card" data-testid="auto-companion-pro-settings">
          <legend>{t('auto-companion-priorities-title')}</legend>
          <p class="help-text card-intro-desc">{t('auto-companion-priorities-desc')}</p>

          <div class="priorities-list" data-testid="priorities-list">
            {#each autoPriorities as role, i (role)}
              <div class="priority-item" data-role={role}>
                <span class="priority-rank">#{i + 1}</span>
                <span class="priority-label">{t(`layout-${role}`)}</span>
                <div class="priority-btns">
                  <button
                    type="button"
                    class="btn-arrow"
                    data-testid="priority-up-{role}"
                    disabled={i === 0}
                    aria-label="Subir {t(`layout-${role}`)}"
                    onclick={() => movePriority(i, 'up')}
                  >▲</button>
                  <button
                    type="button"
                    class="btn-arrow"
                    data-testid="priority-down-{role}"
                    disabled={i === autoPriorities.length - 1}
                    aria-label="Bajar {t(`layout-${role}`)}"
                    onclick={() => movePriority(i, 'down')}
                  >▼</button>
                </div>
              </div>
            {/each}
          </div>

          <div class="field-actions">
            <button
              type="button"
              class="btn-reset-priorities"
              data-testid="btn-reset-priorities"
              onclick={resetPriorities}
            >
              {t('auto-companion-priorities-reset')}
            </button>
          </div>

          <div class="toggle-group linger-slider-group">
            <label class="field-row">
              <span>{t('auto-companion-linger-label')}: <strong class="val-gold font-mono">{lingerSeconds} s</strong></span>
              <input
                type="range"
                min="0"
                max="60"
                step="5"
                data-testid="auto-linger-slider"
                bind:value={lingerSeconds}
                onchange={() => { prefs.autoCompanionLingerSeconds = lingerSeconds; saveConfig(); }}
              />
            </label>
            <p class="help-text">{t('auto-companion-linger-help')}</p>
          </div>

          <label class="field-row">
            <span>{t('auto-companion-predictive-label')}</span>
            <input
              type="checkbox"
              role="switch"
              class="hud-switch"
              data-testid="auto-predictive-switch"
              bind:checked={predictive}
              onchange={() => { prefs.autoCompanionPredictive = predictive; saveConfig(); }}
            />
          </label>

          <label class="field-row">
            <span>{t('auto-companion-sound-cues')}</span>
            <input
              type="checkbox"
              role="switch"
              class="hud-switch"
              data-testid="auto-sound-switch"
              bind:checked={soundCues}
              onchange={() => { prefs.autoCompanionSoundCues = soundCues; saveConfig(); }}
            />
          </label>

          <label class="field-row">
            <span>{t('auto-companion-voice-cues')}</span>
            <input
              type="checkbox"
              role="switch"
              class="hud-switch"
              data-testid="auto-voice-switch"
              bind:checked={voiceCues}
              onchange={() => { prefs.autoCompanionVoiceCues = voiceCues; saveConfig(); }}
            />
          </label>
        </fieldset>

        <fieldset class="settings-card minerals" data-testid="mining-interest">
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
        </fieldset>

        <fieldset class="settings-card minerals engineering-rolls" data-testid="engineering-rolls">
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
      </div>

      <!-- Columna 2: Copiloto Auditivo, Asistencia por Voz e Integración, y Emparejamiento QR -->
      <div class="settings-col">
        <fieldset class="settings-card minerals tts-section" data-testid="tts-settings">
          <legend>{t('settings-tts-title')}</legend>
          <p class="help-text">{t('settings-tts-description')}</p>

          <label class="field-row">
            <span>{t('settings-tts-enabled')}</span>
            <input
              type="checkbox"
              role="switch"
              class="hud-switch"
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
        <fieldset class="settings-card tts-section voice-assistant-section">
          <legend>🎙️ {t('voice-assistant-title')}</legend>

          <div class="field-row voice-toggle-row">
            <label for="voice-assistant-toggle" class="voice-toggle-label">
              <span class="toggle-title">{t('voice-assistant-enable')}</span>
              <small class="field-hint">{t('voice-assistant-enable-desc')}</small>
            </label>
            <input
              id="voice-assistant-toggle"
              type="checkbox"
              role="switch"
              class="hud-switch"
              data-testid="voice-assistant-toggle"
              checked={voiceAssistant.enabled}
              onchange={(e) => voiceAssistant.setEnabled(e.currentTarget.checked)}
            />
          </div>

          {#if voiceAssistant.enabled}
            <div class="field-stacked">
              <label for="voice-engine-select" class="field-label">
                {t('voice-assistant-engine')}
              </label>
              <select
                id="voice-engine-select"
                class="full-width"
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

        <!-- Soporte, Feedback y Comunidad -->
        <fieldset class="settings-card feedback-card" data-testid="settings-feedback-card">
          <legend>💬 {t('settings-feedback-title')}</legend>
          <p class="feedback-desc">{t('settings-feedback-desc')}</p>
          <div class="feedback-actions">
            <button
              type="button"
              class="feedback-btn-primary"
              data-testid="github-feedback-btn"
              onclick={openGitHubFeedback}
            >
              🚀 {t('settings-feedback-btn')}
            </button>
            <a
              href="https://github.com/Danikuso/TheMultipurposeCompanion"
              target="_blank"
              rel="noopener noreferrer"
              class="feedback-btn-secondary"
              data-testid="github-repo-link"
            >
              ⭐ {t('settings-feedback-repo-btn')}
            </a>
          </div>
        </fieldset>
      </div>
    </div>
  </div>

  <div class="settings-accounts-pane" data-testid="settings-accounts-pane">
    <fieldset class="settings-card accounts-wrapper-card">
      <legend>📡 {t('panel-accounts')}</legend>
      <AccountsSection />
    </fieldset>
  </div>
</div>
</div>
</Panel>

<style>
  .settings-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
    max-width: 100%;
  }

  .settings-two-panes {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1.25rem;
    align-items: start;
    width: 100%;
  }

  @media (min-width: 1280px) {
    .settings-two-panes {
      grid-template-columns: minmax(460px, 1fr) minmax(480px, 1.15fr);
    }
  }

  @media (min-width: 1680px) {
    .settings-two-panes {
      grid-template-columns: minmax(520px, 1fr) minmax(580px, 1.25fr);
    }
  }

  .settings-main-pane {
    min-width: 0;
  }

  .settings-accounts-pane {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .accounts-wrapper-card {
    padding: 0.85rem 1rem;
    background: transparent;
    border: none;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 1.25rem;
    align-items: start;
  }

  .settings-col {
    display: flex;
    flex-direction: column;
    gap: 1.1rem;
  }

  .settings-card {
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    padding: 0.85rem 1rem;
    border-radius: 6px;
    background: var(--hud-surface-card, rgba(20, 25, 35, 0.5));
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .settings-card legend {
    font-weight: 700;
    font-size: 0.88rem;
    color: var(--hud-primary, #ff7100);
    padding: 0 0.4rem;
    letter-spacing: 0.03em;
  }

  .auto-companion-card .card-intro-desc {
    margin: 0;
    font-size: 0.78rem;
    color: var(--hud-muted);
  }

  .priorities-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.25rem;
  }

  .priority-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.65rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    gap: 0.5rem;
  }

  .priority-rank {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--hud-accent, #00d4b2);
    min-width: 1.6rem;
  }

  .priority-label {
    flex: 1;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--hud-text);
  }

  .priority-btns {
    display: flex;
    gap: 0.25rem;
  }

  .btn-arrow {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    color: var(--hud-text);
    cursor: pointer;
    font-size: 0.65rem;
    flex-shrink: 0;
    transition: all 120ms ease;
  }

  .btn-arrow:hover:not(:disabled) {
    background: var(--hud-surface-active, rgba(0, 212, 178, 0.15));
    border-color: var(--hud-border);
    color: var(--hud-accent);
  }

  .btn-arrow:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .field-actions {
    display: flex;
    margin-top: 0.25rem;
  }

  .btn-reset-priorities {
    padding: 0.35rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    border-radius: 4px;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    color: var(--hud-muted);
    cursor: pointer;
    transition: all 120ms ease;
  }

  .btn-reset-priorities:hover {
    color: var(--hud-text);
    border-color: var(--hud-border);
  }

  .linger-slider-group {
    margin-top: 0.5rem;
  }

  .field-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .toggle-group {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  select {
    background: var(--hud-surface-alt, #151a29);
    color: var(--hud-text);
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.15));
    padding: 0.4rem 0.65rem;
    border-radius: 4px;
    font-size: 0.85rem;
    outline: none;
    transition: border-color 0.15s ease;
  }

  select:focus {
    border-color: var(--hud-primary, #ff7100);
  }

  .minerals {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .mineral {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .engineering-rolls input {
    background: var(--hud-surface-alt, #151a29);
    color: var(--hud-text);
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.15));
    border-radius: 4px;
    padding: 0.35rem 0.6rem;
    width: 60px;
  }

  .help-text {
    margin: 0;
    font-size: 0.78rem;
    color: var(--hud-muted);
    line-height: 1.35;
  }

  .tts-section {
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    padding: 0.85rem 1rem;
    border-radius: 6px;
    background: var(--hud-surface-card, rgba(20, 25, 35, 0.5));
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .tts-section legend {
    font-weight: 700;
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
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--hud-primary, #ff7100);
  }

  .channels-grid {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
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
    background: var(--hud-surface-alt, #151a29);
    color: var(--hud-primary, #ff7100);
    border: 1px solid var(--hud-border);
    padding: 0.4rem 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.82rem;
    font-weight: 600;
    transition: all 0.15s ease;
  }

  .btn-test-voice:hover:not(:disabled) {
    background: var(--hud-primary, #ff7100);
    color: #000;
  }

  .btn-test-voice:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  /* Asistencia por Voz («TMC Glass HUD») */
  .voice-assistant-section {
    border-color: rgba(0, 212, 178, 0.28);
  }

  .voice-assistant-section legend {
    color: var(--hud-cyan, #00d4b2);
  }

  .voice-assistant-section select:focus {
    border-color: var(--hud-cyan, #00d4b2);
  }

  .voice-toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1.25rem;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid rgba(0, 212, 178, 0.15);
  }

  .voice-toggle-label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    cursor: pointer;
    flex: 1;
  }

  .toggle-title {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--hud-text);
  }

  .field-hint {
    font-size: 0.78rem;
    color: var(--hud-muted);
    line-height: 1.35;
  }

  input[type="checkbox"]:not(.hud-switch) {
    flex-shrink: 0;
    width: 1.1rem;
    height: 1.1rem;
    cursor: pointer;
    accent-color: var(--hud-cyan, #00d4b2);
  }

  .hud-switch {
    appearance: none;
    -webkit-appearance: none;
    width: 38px;
    height: 20px;
    min-width: 38px;
    max-width: 38px;
    background: rgba(255, 255, 255, 0.15);
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.25));
    border-radius: 10px;
    position: relative;
    cursor: pointer;
    outline: none;
    flex-shrink: 0;
    transition: all 150ms ease;
    margin: 0;
    box-sizing: border-box;
  }

  .hud-switch:hover {
    border-color: rgba(255, 255, 255, 0.45);
  }

  .hud-switch::before {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #ffffff;
    transition: transform 150ms ease;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }

  .hud-switch:checked {
    background: var(--hud-cyan, #00d4b2);
    border-color: var(--hud-cyan, #00d4b2);
    box-shadow: 0 0 8px rgba(0, 212, 178, 0.35);
  }

  .hud-switch:checked::before {
    transform: translateX(18px);
  }

  .field-stacked {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.2rem;
  }

  .field-label {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--hud-muted);
  }

  select.full-width {
    width: 100%;
    max-width: 100%;
  }

  .whisper-status-card {
    padding: 0.75rem 0.9rem;
    background: rgba(0, 212, 178, 0.05);
    border: 1px solid rgba(0, 212, 178, 0.22);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    margin: 0.25rem 0;
  }

  .whisper-status-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .whisper-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--hud-cyan, #00d4b2);
  }

  .whisper-desc {
    margin: 0;
    font-size: 0.76rem;
    color: var(--hud-muted);
    line-height: 1.4;
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
    padding: 0.35rem 0.75rem;
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

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.2rem 0.55rem;
    border-radius: 3px;
    font-size: 0.72rem;
    font-weight: 700;
    font-family: var(--font-mono);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .badge-success {
    background: rgba(34, 197, 94, 0.15);
    border: 1px solid rgba(34, 197, 94, 0.4);
    color: var(--hud-success);
  }

  .badge-warning {
    background: rgba(245, 158, 11, 0.15);
    border: 1px solid rgba(245, 158, 11, 0.4);
    color: var(--hud-warning);
  }

  .badge-muted {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--hud-muted);
  }

  .voice-feedback-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.75rem;
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
    padding: 0.8rem 0.9rem;
    background: var(--hud-surface-alt, rgba(20, 25, 35, 0.7));
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .voiceattack-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .va-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--hud-text);
  }

  .va-desc {
    margin: 0;
    font-size: 0.76rem;
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
    border: 1px solid rgba(0, 212, 178, 0.35);
    padding: 0.45rem 0.85rem;
    border-radius: 4px;
    text-decoration: none;
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .btn-download-vap:hover {
    background: var(--hud-cyan, #00d4b2);
    color: #000;
  }

  .feedback-card {
    border-color: rgba(255, 113, 0, 0.25);
    background: linear-gradient(180deg, rgba(20, 25, 35, 0.6) 0%, rgba(255, 113, 0, 0.04) 100%);
  }

  .feedback-desc {
    margin: 0;
    font-size: 0.8rem;
    color: var(--hud-muted, #8ea1b4);
    line-height: 1.45;
  }

  .feedback-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    align-items: center;
    margin-top: 0.35rem;
  }

  .feedback-btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    background: var(--hud-primary, #ff7100);
    color: #000;
    font-weight: 700;
    font-size: 0.82rem;
    padding: 0.5rem 0.95rem;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    transition: filter 0.15s ease, transform 0.1s ease;
    box-shadow: 0 0 10px rgba(255, 113, 0, 0.3);
  }

  .feedback-btn-primary:hover {
    filter: brightness(1.15);
    transform: translateY(-1px);
  }

  .feedback-btn-secondary {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(255, 255, 255, 0.06);
    color: var(--hud-text, #e2e8f0);
    font-weight: 600;
    font-size: 0.82rem;
    padding: 0.5rem 0.85rem;
    border-radius: 4px;
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.15));
    text-decoration: none;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .feedback-btn-secondary:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: var(--hud-primary, #ff7100);
  }
</style>
