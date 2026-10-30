<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { connect, topic } from './lib/ws.svelte';
  import { diag, diagOn } from './lib/diag.svelte';
  import { t } from './lib/i18n';
  import { loadPrefs, prefs, prefsFallback, readPlacement, savePlacement, toggleTacticalLock } from './lib/prefs.svelte';
  import { modules } from './modules/registry';
  import Settings from './settings/Settings.svelte';
  import Grid from './shell/Grid.svelte';
  import ImportProgress from './shell/ImportProgress.svelte';
  import Sidebar from './shell/Sidebar.svelte';
  import Toasts from './shell/Toasts.svelte';
  import TopBar from './shell/TopBar.svelte';
  import { DEFAULT_PRIORITIES, autoLayout, contextPanel, layouts, nextPin } from './shell/layouts';

  let pin = $state<string | null>(null);
  let view = $state<string | null>(null);
  const OFF_MARK = ' off';
  let lastAuto: string | null = null;
  let manual = $state<string | null>(null);
  let manualBody = $state<string | null>(null);

  const gameTopic = $derived(topic<{ running?: boolean; replaying?: boolean }>('core.game'));
  const isReplaying = $derived(gameTopic?.replaying === true);
  const isStable = $derived(!isReplaying);

  const liveModules = $derived(topic<{ id: string; relevance: string }[]>('core.modules') ?? []);
  const location = $derived(topic<{ system?: string; system_address?: number; body?: string | null }>('core.location'));
  const exoBody = $derived(topic<{ system_address?: number; body_id?: number; body?: string | null }>('exobiology.body'));
  const matchingExoBody = $derived(!location || location.system_address === exoBody?.system_address ? exoBody : null);
  const bodyIdentity = $derived(JSON.stringify([
    location?.system_address ?? location?.system ?? matchingExoBody?.system_address,
    matchingExoBody?.body_id != null ? { id: matchingExoBody.body_id }
      : { name: location?.body ?? matchingExoBody?.body ?? null },
  ]));
  const auto = $derived(autoLayout(liveModules, null, prefs.autoCompanionPriorities, prefs.locked));

  function playGlassHudChime() {
    try {
      const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (!AudioCtx) return;
      const ctx = new AudioCtx();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(880, ctx.currentTime);
      osc.frequency.exponentialRampToValueAtTime(1320, ctx.currentTime + 0.08);
      gain.gain.setValueAtTime(0.04, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.12);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.12);
    } catch {
      // AudioContext no disponible o bloqueado por política de autoplay
    }
  }

  function playVoiceCue(layoutId: string) {
    try {
      if (!('speechSynthesis' in window)) return;
      let key = '';
      if (layoutId === 'combat') key = 'voice-auto-combat';
      else if (layoutId === 'mining') key = 'voice-auto-mining';
      else if (layoutId === 'trade') key = 'voice-auto-trade';
      else if (layoutId === 'engineering') key = 'voice-auto-engineering';
      if (!key) return;
      const text = t(key);
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.rate = 1.05;
      window.speechSynthesis.speak(utterance);
    } catch {
      // Voz TTS no disponible
    }
  }

  function playCockpitCue(layoutId: string) {
    if (prefs.autoCompanionSoundCues) {
      playGlassHudChime();
    }
    if (prefs.autoCompanionVoiceCues) {
      playVoiceCue(layoutId);
    }
  }

  // Auto-companion apagado o durante arranque: la disposición y el panel de contexto se quedan estables.
  let heldLayout = $state('exploration');
  let heldContext = $state('exploration');
  const follow = $derived(prefs.loaded && prefs.autoCompanion);
  const shownLayout = $derived(follow && isStable ? (auto ?? 'exploration') : heldLayout);
  const activeTab = $derived(view ?? pin ?? `layout:${shownLayout}`);
  const activeLayout = $derived(layouts.find((layout) => activeTab === `layout:${layout.id}`));
  const autoManual = $derived(manualBody === bodyIdentity ? manual : null);
  const layoutContext = $derived({
    modules: liveModules,
    manual: follow ? autoManual : (manual ?? heldContext),
  });
  const autoContext = $derived(contextPanel(liveModules, autoManual));
  const visibleSlots = $derived(activeLayout?.slots.filter((slot) => !slot.when || slot.when(layoutContext)) ?? []);
  const selectedContext = $derived(follow ? autoContext : contextPanel(liveModules, layoutContext.manual));
  const activeModule = $derived(modules.find((m) => m.id === activeTab) ?? modules[0]);

  $effect(() => {
    if (follow && isStable) {
      heldLayout = auto ?? 'exploration';
      heldContext = autoContext;
    }
  });

  $effect(() => {
    if (!follow || !isStable) {
      if (!follow) lastAuto = OFF_MARK; // al reactivar, la disposición automática se aplica enseguida
      return;
    }
    const a = auto;
    const current = untrack(() => pin);
    const next = nextPin(lastAuto, a, current);
    const changed = lastAuto !== OFF_MARK && lastAuto !== null && a !== null && a !== lastAuto;
    lastAuto = a;
    if (next !== current) pin = next;
    if (changed) {
      playCockpitCue(a);
    }
  });

  // Con Auto-companion apagado se recuerda dónde lo dejó el usuario para el próximo arranque.
  $effect(() => {
    if (!prefs.loaded || prefs.autoCompanion) return;
    savePlacement({ layout: pin?.startsWith('layout:') ? pin.slice(7) : heldLayout, context: layoutContext.manual });
  });

  $effect(() => {
    if (!diagOn()) return;
    const ax = topic<{ active?: boolean; presence?: boolean } | null>('combat.ax');
    diag('ui.app.layout', {
      tab: activeTab,
      layout: activeLayout?.id ?? 'none',
      pinned: pin !== null,
      auto_companion: prefs.autoCompanion,
      slot: visibleSlots.map((s) => s.id),
      defined_slots: activeLayout?.slots.length ?? 0,
      module: liveModules.map((m) => m.id),
      relevance: liveModules.map((m) => String(m.relevance).toLowerCase()),
      context: selectedContext,
      manual_context: layoutContext.manual ?? 'none',
      ax_active: ax?.active === true,
      ax_presence: ax?.presence === true,
      cz_present: (topic<unknown>('combat.conflict_zone') ?? null) !== null,
    });
  });

  function selectTab(id: string) {
    if (id === 'accounts') {
      view = 'settings';
      return;
    }
    if (id.startsWith('layout:')) { pin = id; view = null; } else view = id;
  }

  $effect(() => {
    if (follow && manualBody !== bodyIdentity) {
      manual = null;
      manualBody = bodyIdentity;
    }
  });

  function selectContext(id: string) {
    manualBody = bodyIdentity;
    manual = id;
  }

  onMount(() => {
    connect();
    fetch('/api/config').then((res) => (res.ok ? res.json() : null)).then((cfg) => {
      if (!cfg) return prefsFallback();
      if (cfg.auto_companion === false) {
        const saved = readPlacement();
        if (saved.layout && layouts.some((l) => l.id === saved.layout)) heldLayout = saved.layout;
        if (saved.context === 'exploration' || saved.context === 'exobiology') heldContext = saved.context;
      }
      loadPrefs(cfg);
    }).catch(prefsFallback);

    function handleKeydown(e: KeyboardEvent) {
      if (e.altKey && (e.key === 'l' || e.key === 'L')) {
        e.preventDefault();
        toggleTacticalLock();
      }
    }
    window.addEventListener('keydown', handleKeydown);

    const timer = setTimeout(prefsFallback, 3000);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('keydown', handleKeydown);
    };
  });
</script>

<div class="app-shell">
  <TopBar />
  <ImportProgress />
  <div class="app-body">
    <Sidebar {activeTab} onSelect={selectTab} />
    <Grid {activeTab}>
      {#if activeTab === 'settings'}
        <Settings />
      {:else if activeLayout}
        {#each visibleSlots as slot (slot.id)}
          <div class="layout-slot" class:full={slot.span === 'full'}
            style:--slot-span={slot.span === 'full' ? 1 : slot.span} data-layout-slot={slot.id}>
            {#if activeLayout.id === 'exploration' && slot.span === 2}
              <div class="context-controls" role="group" aria-label={t('layout-context')}>
                <button type="button" data-testid="context-bodies" aria-pressed={selectedContext === 'exploration'}
                  onclick={() => selectContext('exploration')}>{t('layout-context-bodies')}</button>
                <button type="button" data-testid="context-exobiology" aria-pressed={selectedContext === 'exobiology'}
                  onclick={() => selectContext('exobiology')}>{t('layout-context-exobiology')}</button>
              </div>
            {/if}
            <slot.panel />
          </div>
        {/each}
      {:else if activeModule}
        {#each activeModule.panels as PanelComp, idx (`${activeModule.id}-${idx}`)}
          <PanelComp />
        {/each}
      {/if}
    </Grid>
  </div>
  <Toasts />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;
  }

  .app-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .layout-slot { min-width: 0; grid-column: 1 / -1; }
  .context-controls { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; }
  .context-controls button { background: var(--hud-surface); color: var(--hud-text); border: 1px solid var(--hud-border); padding: 0.4rem 0.6rem; }
  .context-controls button[aria-pressed='true'] { color: var(--hud-primary); border-color: var(--hud-primary); }
  @media (min-width: 900px) {
    .layout-slot:not(.full) { grid-column: span var(--slot-span); }
  }
</style>
