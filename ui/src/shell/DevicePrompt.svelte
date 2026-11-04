<script lang="ts">
  import { onMount } from 'svelte';
  import { getLocale, t } from '../lib/i18n';
  import { listProfiles, type ProfileSummary } from '../lib/layoutApi';
  import { NAME_MAX, normalizeTabName } from './layoutModel';
  import { createProfile, defaultProfileName, layoutState, type ProfileMode } from './layoutStore.svelte';

  type Choice = 'factory' | 'copy' | 'claim';

  let name = $state(defaultProfileName());
  let choice = $state<Choice>('factory');
  let profiles = $state<ProfileSummary[]>([]);
  let copyFrom = $state('');
  let claimId = $state('');
  let busy = $state(false);
  let failed = $state(false);

  const canClaim = $derived(layoutState.deviceKind !== 'desktop');
  const valid = $derived(normalizeTabName(name) !== null);

  onMount(async () => {
    let list: ProfileSummary[] = [];
    try { list = await listProfiles(); } catch { /* servidor caído: sin perfiles que ofrecer */ }
    // Más reciente primero; las fechas ISO se ordenan como texto.
    profiles = [...list].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    copyFrom = claimId = profiles[0]?.device_id ?? '';
  });

  function label(p: ProfileSummary): string {
    const d = new Date(p.updated_at);
    const date = Number.isNaN(d.getTime()) ? '' : ` · ${d.toLocaleDateString(getLocale())}`;
    return `${p.name}${date}`;
  }

  async function run(finalName: string, mode: ProfileMode) {
    busy = true;
    failed = false;
    const ok = await createProfile(finalName, mode);
    busy = false;
    failed = !ok;
  }

  function accept(e: Event) {
    e.preventDefault();
    const n = normalizeTabName(name);
    if (n === null || busy) return;
    const mode: ProfileMode = choice === 'copy' ? { kind: 'copy', from: copyFrom }
      : choice === 'claim' ? { kind: 'claim', id: claimId } : { kind: 'factory' };
    void run(n, mode);
  }

  function dismiss() {
    if (!busy) void run(defaultProfileName(), { kind: 'factory' });
  }
</script>

<form class="device-prompt" data-testid="device-prompt" aria-label={t('layout-device-title')} onsubmit={accept}>
  <div class="head">
    <div class="head-text">
      <strong>{t('layout-device-title')}</strong>
      <span class="hint">{t('layout-device-hint')}</span>
    </div>
    <button type="button" class="close" data-testid="device-dismiss" aria-label={t('layout-device-dismiss')}
      title={t('layout-device-dismiss')} disabled={busy} onclick={dismiss}>×</button>
  </div>

  <label class="field">
    <span>{t('layout-device-name')}</span>
    <input type="text" data-testid="device-name" maxlength={NAME_MAX} bind:value={name} />
  </label>

  <div class="options">
    <label class="opt">
      <input type="radio" name="device-mode" data-testid="device-mode-factory" value="factory" bind:group={choice} />
      <span>{t('layout-device-factory')}</span>
    </label>
    {#if profiles.length > 0}
      <div class="opt-block">
        <label class="opt">
          <input type="radio" name="device-mode" data-testid="device-mode-copy" value="copy" bind:group={choice} />
          <span>{t('layout-device-copy')}</span>
        </label>
        {#if choice === 'copy'}
          <select data-testid="device-copy-select" aria-label={t('layout-device-copy')} bind:value={copyFrom}>
            {#each profiles as p (p.device_id)}<option value={p.device_id}>{label(p)}</option>{/each}
          </select>
        {/if}
      </div>
      {#if canClaim}
        <div class="opt-block">
          <label class="opt">
            <input type="radio" name="device-mode" data-testid="device-mode-claim" value="claim" bind:group={choice} />
            <span>{t('layout-device-claim')}</span>
          </label>
          {#if choice === 'claim'}
            <select data-testid="device-claim-select" aria-label={t('layout-device-claim')} bind:value={claimId}>
              {#each profiles as p (p.device_id)}<option value={p.device_id}>{label(p)}</option>{/each}
            </select>
          {/if}
        </div>
      {/if}
    {/if}
  </div>

  {#if failed}<p class="error" role="alert" data-testid="device-error">{t('layout-device-error')}</p>{/if}

  <div class="form-actions">
    <button type="submit" class="accept" data-testid="device-accept" disabled={busy || !valid}>{t('layout-device-accept')}</button>
  </div>
</form>

<style>
  .device-prompt {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-width: 0;
    padding: 0.75rem 1rem;
    background: var(--hud-surface);
    border: 1px solid var(--hud-border);
    border-radius: 6px;
    color: var(--hud-text);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .head-text {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .hint { opacity: 0.75; font-size: 0.85rem; }

  .close {
    flex-shrink: 0;
    padding: 0 0.5rem;
    line-height: 1.6;
    background: transparent;
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  .field { display: flex; flex-wrap: wrap; align-items: center; gap: 0.5rem; }
  .field input[type='text'] { flex: 1 1 12rem; min-width: 0; }

  .options { display: flex; flex-direction: column; gap: 0.5rem; }
  .opt-block { display: flex; flex-wrap: wrap; align-items: center; gap: 0.5rem 0.75rem; min-width: 0; }
  .opt { display: flex; align-items: center; gap: 0.4rem; min-width: 0; }
  .opt input { flex-shrink: 0; }
  .opt-block select { flex: 1 1 14rem; min-width: 0; max-width: 100%; }

  .error { margin: 0; color: var(--hud-danger, #ff6b6b); font-size: 0.85rem; }

  .form-actions { display: flex; flex-wrap: wrap; gap: 0.5rem; }
  .accept {
    flex: 1;
    padding: 0.35rem 0.75rem;
    background: var(--hud-surface);
    color: var(--hud-text);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
  }

  button:disabled { opacity: 0.5; }
</style>
