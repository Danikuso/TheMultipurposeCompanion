<script lang="ts">
  import { onMount } from 'svelte';
  import { askConfirm } from '../lib/confirm';
  import { diag } from '../lib/diag.svelte';
  import { getLocale, t } from '../lib/i18n';
  import { copyProfile, deleteProfile, listProfiles, renameProfile, type ProfileSummary } from '../lib/layoutApi';
  import { deleteCustomTab, moveCustomTab, normalizeTabName, NAME_MAX } from '../shell/layoutModel';
  import { initLayouts, layoutState, mutateAndSave } from '../shell/layoutStore.svelte';

  let profiles = $state<ProfileSummary[]>([]);
  let busy = $state(false);
  let failed = $state(false);
  let renamingId = $state<string | null>(null);
  let renameValue = $state('');

  const locked = $derived(layoutState.editing || layoutState.offline || busy);
  const renameValid = $derived(normalizeTabName(renameValue) !== null);
  const ordered = $derived.by(() => {
    const cur = layoutState.deviceId;
    const rest = profiles
      .filter((p) => p.device_id !== cur)
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    return [...profiles.filter((p) => p.device_id === cur), ...rest];
  });
  const customList = $derived(
    layoutState.doc.customOrder.flatMap((id) => layoutState.doc.custom.find((c) => c.id === id) ?? []),
  );

  function dateOf(p: ProfileSummary): string {
    const d = new Date(p.updated_at);
    return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString(getLocale());
  }

  async function refresh(): Promise<void> {
    try {
      const list = await listProfiles();
      profiles = Array.isArray(list) ? list : [];
    } catch {
      failed = true;
    }
  }

  /** Ejecuta una acción mutadora: bloquea los botones, recarga la lista y muestra un error genérico si falla. */
  async function run(action: () => Promise<boolean | void>): Promise<void> {
    if (locked) return;
    busy = true;
    failed = false;
    try {
      if ((await action()) === false) failed = true;
    } catch {
      diag('ui.layout.save', { settings: true });
      failed = true;
    } finally {
      busy = false;
    }
    await refresh();
  }

  function startRename(p: ProfileSummary): void {
    renamingId = p.device_id;
    renameValue = p.name;
  }

  const saveRename = () => run(async () => {
    const id = renamingId;
    const name = normalizeTabName(renameValue);
    if (id === null || name === null) return false;
    await renameProfile(id, name);
    if (id === layoutState.deviceId) layoutState.profileName = name;
    renamingId = null;
  });

  function onRenameKey(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (renameValid) void saveRename();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      renamingId = null;
    }
  }

  const copyHere = (p: ProfileSummary) => run(async () => {
    const cur = layoutState.deviceId;
    if (cur === null || !askConfirm(t('layout-settings-copy-confirm', { name: p.name }))) return;
    await copyProfile(cur, p.device_id, layoutState.profileName ?? p.name);
    await initLayouts();
  });

  const remove = (p: ProfileSummary) => run(async () => {
    if (!askConfirm(t('layout-settings-delete-confirm', { name: p.name }))) return;
    await deleteProfile(p.device_id);
  });

  const resetAll = () => run(() => {
    if (!askConfirm(t('layout-settings-reset-all-confirm'))) return Promise.resolve();
    return mutateAndSave((d) => ({ ...d, tabs: {} }));
  });

  const move = (id: string, delta: -1 | 1) => run(() => mutateAndSave((d) => moveCustomTab(d, id, delta)));

  const removeTab = (id: string) => run(() => {
    if (!askConfirm(t('layout-custom-delete-confirm'))) return Promise.resolve();
    return mutateAndSave((d) => deleteCustomTab(d, id));
  });

  onMount(refresh);
</script>

<div class="layout-settings" data-testid="layout-settings">
  {#if layoutState.editing}
    <p class="hint" data-testid="layout-settings-editing-hint">{t('layout-settings-editing-hint')}</p>
  {:else if layoutState.offline}
    <p class="hint" data-testid="layout-settings-offline-hint">{t('layout-device-offline-hint')}</p>
  {/if}
  {#if failed}<p class="error" role="alert" data-testid="layout-settings-error">{t('layout-settings-error')}</p>{/if}

  <h4>{t('layout-settings-profiles')}</h4>
  <ul class="profiles">
    {#each ordered as p (p.device_id)}
      {@const current = p.device_id === layoutState.deviceId}
      <li class="profile" data-testid={`profile-row-${p.device_id}`}>
        <div class="profile-head">
          <span class="name">{p.name}</span>
          {#if current}<span class="pill" data-testid={`profile-current-${p.device_id}`}>{t('layout-device-this')}</span>{/if}
          <span class="date">{dateOf(p)}</span>
        </div>
        {#if renamingId === p.device_id}
          <div class="form-actions">
            <input
              type="text"
              class="rename-input"
              data-testid="profile-rename-input"
              aria-label={t('layout-settings-rename')}
              maxlength={NAME_MAX}
              bind:value={renameValue}
              onkeydown={onRenameKey}
            />
            <button type="button" data-testid="profile-rename-save" disabled={locked || !renameValid} onclick={saveRename}>{t('layout-settings-rename-save')}</button>
            <button type="button" data-testid="profile-rename-cancel" onclick={() => (renamingId = null)}>{t('layout-edit-cancel')}</button>
          </div>
        {:else}
          <div class="form-actions">
            <button type="button" data-testid={`profile-rename-${p.device_id}`} disabled={locked} onclick={() => startRename(p)}>{t('layout-settings-rename')}</button>
            {#if !current}
              <button type="button" data-testid={`profile-copy-${p.device_id}`} disabled={locked} onclick={() => copyHere(p)}>{t('layout-settings-copy')}</button>
            {/if}
            <button
              type="button"
              data-testid={`profile-delete-${p.device_id}`}
              disabled={locked || current}
              title={current ? t('layout-settings-delete-current') : ''}
              onclick={() => remove(p)}
            >{t('layout-settings-delete')}</button>
          </div>
        {/if}
      </li>
    {/each}
  </ul>

  <div class="form-actions">
    <button type="button" data-testid="layout-reset-all" disabled={locked} onclick={resetAll}>{t('layout-settings-reset-all')}</button>
  </div>

  <h4 data-testid="layout-settings-tabs-title">{t('layout-settings-tabs')}</h4>
  {#if customList.length === 0}
    <p class="hint">{t('layout-settings-tabs-empty')}</p>
  {:else}
    <ul class="profiles">
      {#each customList as c, i (c.id)}
        <li class="profile" data-testid={`custom-row-${c.id}`}>
          <div class="profile-head"><span class="name">{c.name}</span></div>
          <div class="form-actions">
            <button type="button" data-testid={`custom-up-${c.id}`} disabled={locked || i === 0} aria-label={t('layout-settings-move-up')} title={t('layout-settings-move-up')} onclick={() => move(c.id, -1)}>↑</button>
            <button type="button" data-testid={`custom-down-${c.id}`} disabled={locked || i === customList.length - 1} aria-label={t('layout-settings-move-down')} title={t('layout-settings-move-down')} onclick={() => move(c.id, 1)}>↓</button>
            <button type="button" data-testid={`custom-delete-${c.id}`} disabled={locked} onclick={() => removeTab(c.id)}>{t('layout-custom-delete')}</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .layout-settings { display: flex; flex-direction: column; gap: 0.6rem; min-width: 0; color: var(--hud-text); }
  h4 { margin: 0.2rem 0 0; font-size: 0.82rem; color: var(--hud-muted); letter-spacing: 0.03em; }
  .hint { margin: 0; font-size: 0.8rem; color: var(--hud-muted); }
  .error { margin: 0; font-size: 0.85rem; color: var(--hud-danger, #ff6b6b); }
  .profiles { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.5rem; }
  .profile {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
    padding: 0.5rem 0.65rem;
    border: 1px solid var(--hud-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 6px;
    background: var(--hud-surface, rgba(20, 25, 35, 0.5));
  }
  .profile-head { display: flex; flex-wrap: wrap; align-items: center; gap: 0.25rem 0.5rem; min-width: 0; }
  .name { font-weight: 600; min-width: 0; overflow-wrap: anywhere; }
  .date { margin-left: auto; font-size: 0.78rem; color: var(--hud-muted); flex-shrink: 0; }
  .pill {
    flex-shrink: 0;
    padding: 0 0.5rem;
    font-size: 0.72rem;
    line-height: 1.5;
    border: 1px solid var(--hud-primary, #ff7100);
    border-radius: 999px;
    color: var(--hud-primary, #ff7100);
  }
  .form-actions { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .form-actions button {
    flex: 1 1 auto;
    padding: 0.3rem 0.7rem;
    background: var(--hud-surface);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }
  .rename-input { flex: 1 1 12rem; min-width: 0; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
