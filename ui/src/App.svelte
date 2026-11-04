<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { connect, topic } from './lib/ws.svelte';
  import { diag, diagOn } from './lib/diag.svelte';
  import { t } from './lib/i18n';
  import { loadPrefs, prefs, prefsFallback, readPlacement, savePlacement, toggleTacticalLock } from './lib/prefs.svelte';
  import Settings from './settings/Settings.svelte';
  import Grid from './shell/Grid.svelte';
  import ImportProgress from './shell/ImportProgress.svelte';
  import Sidebar from './shell/Sidebar.svelte';
  import Toasts from './shell/Toasts.svelte';
  import TopBar from './shell/TopBar.svelte';
  import UpdateModal from './shell/UpdateModal.svelte';
  import { fetchUpdateStatus } from './lib/updater.svelte';
  import { tabKeyFor } from './shell/cards';
  import CardHost, { isCardVisible, tabPlacements } from './shell/CardHost.svelte';
  import { beginEdit, customTab, editDoc, initLayouts, layoutState } from './shell/layoutStore.svelte';
  import { askConfirm } from './lib/confirm';
  import CustomTabDialog from './shell/CustomTabDialog.svelte';
  import { pushToast } from './shell/Toasts.svelte';
  import { createCustomTab, deleteCustomTab, updateCustomTab } from './shell/layoutModel';
  import { cancelEditWithDiag } from './shell/editDiag';
  import { activeDrag } from './shell/pointerDrag';
  import GridHeader from './shell/GridHeader.svelte';
  import EditToolbar from './shell/EditToolbar.svelte';
  import { modules } from './modules/registry';
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
  // Mientras se edita la disposición, Auto-companion queda en suspenso: la pestaña no cambia bajo el cursor.
  const following = $derived(follow && isStable && !layoutState.editing);
  const shownLayout = $derived(following ? (auto ?? 'exploration') : heldLayout);
  const activeTab = $derived(view ?? pin ?? `layout:${shownLayout}`);
  const activeLayout = $derived(layouts.find((layout) => activeTab === `layout:${layout.id}`));
  const autoManual = $derived(manualBody === bodyIdentity ? manual : null);
  const layoutContext = $derived({
    modules: liveModules,
    manual: follow ? autoManual : (manual ?? heldContext),
  });
  const autoContext = $derived(contextPanel(liveModules, autoManual));
  const tabKey = $derived(tabKeyFor(activeTab));
  const headerTitleKey = $derived(activeLayout?.titleKey ?? modules.find((m) => m.id === activeTab)?.titleKey);
  const headerTitle = $derived(
    tabKey?.startsWith('custom:') ? customTab(tabKey.slice('custom:'.length))?.name : undefined,
  );
  const activeCustom = $derived(tabKey?.startsWith('custom:') ? customTab(tabKey.slice('custom:'.length)) : undefined);
  const tabCards = $derived(tabKey === null ? [] : tabPlacements(tabKey));
  const visibleSlots = $derived(tabCards.filter((p) => isCardVisible(p.id, layoutContext)));
  const selectedContext = $derived(follow ? autoContext : contextPanel(liveModules, layoutContext.manual));

  $effect(() => {
    if (following) {
      heldLayout = auto ?? 'exploration';
      heldContext = autoContext;
    }
  });

  // Una pestaña propia que ya no existe (borrada, cancelada o restaurada de un guardado viejo) devuelve a la automática.
  $effect(() => {
    if (view?.startsWith('custom:') && layoutState.ready && !customTab(view.slice('custom:'.length))) view = null;
  });

  $effect(() => {
    if (!following) {
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
    savePlacement({
      layout: pin?.startsWith('layout:') ? pin.slice(7) : heldLayout,
      context: layoutContext.manual,
      view: view?.startsWith('custom:') ? view : null,
    });
  });

  // Esc aborta el arrastre en curso o, si no hay, cancela la edición; el listener solo existe mientras se edita.
  $effect(() => {
    if (!layoutState.editing) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      const drag = activeDrag();
      if (drag?.active) drag.cancel(); // Esc durante un arrastre solo aborta el arrastre
      else cancelEditWithDiag(tabKey);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
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
      defined_slots: tabCards.length,
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

  let tabDialog = $state<'create' | 'edit' | null>(null);

  function submitTab(name: string, icon: string) {
    const mode = tabDialog;
    tabDialog = null;
    if (mode === 'edit' && activeCustom) {
      const id = activeCustom.id;
      editDoc((d) => updateCustomTab(d, id, { name, icon }));
      return;
    }
    if (!layoutState.editing) beginEdit();
    const id = `c-${crypto.randomUUID()}`;
    let created = false;
    editDoc((d) => {
      const next = createCustomTab(d, name, icon, id);
      created = next !== null;
      return next ?? d;
    });
    if (!created) {
      pushToast(t('layout-custom-limit'), 'Warning', 'layout-custom-limit');
      return;
    }
    selectTab(`custom:${id}`);
    layoutState.catalogRequested = true;
  }

  function deleteActiveTab() {
    const id = activeCustom?.id;
    if (!id || !askConfirm(t('layout-custom-delete-confirm'))) return;
    editDoc((d) => deleteCustomTab(d, id));
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
    void initLayouts();
    fetch('/api/config').then((res) => (res.ok ? res.json() : null)).then((cfg) => {
      if (!cfg) return prefsFallback();
      if (cfg.auto_companion === false) {
        const saved = readPlacement();
        if (saved.layout && layouts.some((l) => l.id === saved.layout)) heldLayout = saved.layout;
        if (saved.context === 'exploration' || saved.context === 'exobiology') heldContext = saved.context;
        if (saved.view) view = saved.view;
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
    const updateTimer = setTimeout(() => {
      void fetchUpdateStatus();
    }, 3500);

    return () => {
      clearTimeout(timer);
      clearTimeout(updateTimer);
      window.removeEventListener('keydown', handleKeydown);
    };
  });
</script>

{#snippet contextControls(cardId: string)}{#if cardId === 'exploration-context'}<div
  class="context-controls" role="group" aria-label={t('layout-context')}>
  <button type="button" data-testid="context-bodies" aria-pressed={selectedContext === 'exploration'}
    onclick={() => selectContext('exploration')}>{t('layout-context-bodies')}</button>
  <button type="button" data-testid="context-exobiology" aria-pressed={selectedContext === 'exobiology'}
    onclick={() => selectContext('exobiology')}>{t('layout-context-exobiology')}</button>
</div>{/if}{/snippet}

{#snippet customActions()}
  <button type="button" data-testid="custom-tab-rename" onclick={() => (tabDialog = 'edit')}>{t('layout-custom-rename')}</button>
  <button type="button" data-testid="custom-tab-delete" onclick={deleteActiveTab}>{t('layout-custom-delete')}</button>
{/snippet}

<div class="app-shell">
  <TopBar />
  <ImportProgress />
  <div class="app-body">
    <Sidebar {activeTab} onSelect={selectTab} onNewTab={() => (tabDialog = 'create')} />
    <Grid {activeTab}>
      {#if activeTab === 'settings'}
        <Settings />
      {:else if tabKey !== null}
        <GridHeader titleKey={headerTitleKey} title={headerTitle} {tabKey} />
        {#if layoutState.editing}
          <EditToolbar {tabKey} extra={activeCustom ? customActions : undefined} />
        {:else if activeCustom && tabCards.length === 0}
          <div class="custom-empty" data-testid="custom-empty">
            <p>{t('layout-custom-empty')}</p>
            <button type="button" data-testid="custom-empty-edit" onclick={() => beginEdit()}>{t('layout-edit-start')}</button>
          </div>
        {/if}
        <CardHost {tabKey} ctx={layoutContext} prefix={contextControls} />
      {/if}
    </Grid>
  </div>
  <Toasts />
  <UpdateModal />
  {#if tabDialog}
    <CustomTabDialog mode={tabDialog}
      initial={tabDialog === 'edit' && activeCustom ? { name: activeCustom.name, icon: activeCustom.icon } : undefined}
      onsubmit={submitTab} onclose={() => (tabDialog = null)} />
  {/if}
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

  .custom-empty {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 1rem;
    color: var(--hud-muted);
  }
  .custom-empty p { margin: 0; }
  .custom-empty button { background: var(--hud-surface); color: var(--hud-primary); border: 1px solid var(--hud-primary); padding: 0.4rem 0.8rem; }

  .context-controls { display: flex; flex-shrink: 0; gap: 0.5rem; margin-bottom: 0.5rem; }
  .context-controls button { background: var(--hud-surface); color: var(--hud-text); border: 1px solid var(--hud-border); padding: 0.4rem 0.6rem; }
  .context-controls button[aria-pressed='true'] { color: var(--hud-primary); border-color: var(--hud-primary); }
</style>
