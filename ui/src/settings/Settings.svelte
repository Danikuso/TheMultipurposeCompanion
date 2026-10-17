<script lang="ts">
  import { onMount } from 'svelte';
  import { getLocale, setLocale, t } from '../lib/i18n';
  import { applyTheme, type HudThemeInput } from '../lib/theme';
  import { topic } from '../lib/ws.svelte';
  import Panel from '../shell/Panel.svelte';
  import Pairing from './Pairing.svelte';

  interface ConfigDto {
    language?: string;
    auto_copy_next_system?: boolean;
    alerts_enabled?: boolean;
    theme_mode?: string;
    server?: {
      port?: number;
      lan?: boolean;
    };
  }

  let language = $state(getLocale());
  let themeMode = $state<'hud' | 'classic' | 'contrast'>('hud');
  let autoCopy = $state(true);
  let lanEnabled = $state(false);
  let fullConfig = $state<ConfigDto>({});

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

  onMount(async () => {
    try {
      const res = await fetch('/api/config');
      if (res.ok) {
        const cfg = (await res.json()) as ConfigDto;
        fullConfig = cfg;
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
    }
  });

  async function saveConfig() {
    setLocale(language);
    applySelectedTheme(themeMode);
    const updated: ConfigDto = {
      ...fullConfig,
      language,
      auto_copy_next_system: autoCopy,
      theme_mode: themeMode,
      server: {
        ...(fullConfig.server ?? {}),
        lan: lanEnabled,
      },
    };
    try {
      const res = await fetch('/api/config', {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(updated),
      });
      if (res.ok) {
        fullConfig = (await res.json()) as ConfigDto;
      }
    } catch {
      // Ignore save error in UI
    }
  }
</script>

<Panel titleKey="nav-settings">
  <div class="settings-form">
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

    <label class="field-row">
      <span>{t('settings-lan')}</span>
      <input type="checkbox" bind:checked={lanEnabled} onchange={saveConfig} />
    </label>
    <p class="help-text">{t('settings-lan-help')}</p>

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

  .help-text {
    margin: 0;
    font-size: 0.8rem;
    color: var(--hud-muted);
  }
</style>
