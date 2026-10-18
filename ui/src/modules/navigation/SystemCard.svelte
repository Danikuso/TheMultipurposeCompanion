<script lang="ts">
  import { t } from '../../lib/i18n';
  import { ageShort, ly, pct, starClassLabel } from '../../lib/format';
  import { getLocale } from '../../lib/i18n';
  import type { Card, Politics } from './types';

  let {
    card,
    big = false,
    remainingJumps = null,
    totalLy = null,
  }: {
    card: Card;
    big?: boolean;
    remainingJumps?: number | null;
    totalLy?: number | null;
  } = $props();

  const DANGER_STATES = ['war', 'civilwar', 'civilunrest', 'outbreak'];

  const remoteOk = $derived(card.remote.status === 'ok' && card.remote.data != null);
  const pol = $derived<Politics | null>(remoteOk ? card.remote.data : card.local_politics);
  const isCache = $derived((card.remote.source ?? '').toLowerCase() === 'cache');
  const cacheAge = $derived.by(() => {
    if (!isCache || !card.remote.fetched_at) return null;
    const ms = Date.now() - Date.parse(card.remote.fetched_at);
    return Number.isNaN(ms) ? null : ageShort(ms / 1000);
  });
  const danger = $derived(
    DANGER_STATES.includes((pol?.faction_state ?? '').replace(/[\s_-]/g, '').toLowerCase()),
  );
  const specialKey = $derived(
    card.special === 'neutron'
      ? 'strip-neutron'
      : card.special === 'white_dwarf'
        ? 'strip-white-dwarf'
        : null,
  );
  const noData = $derived(t('empty-no-data'));
  const discovered = $derived(
    !remoteOk || card.remote.data?.discovered == null
      ? noData
      : card.remote.data.discovered
        ? t('strip-discovered')
        : t('strip-undiscovered'),
  );
  const population = $derived(
    pol?.population == null ? null : new Intl.NumberFormat(getLocale()).format(pol.population),
  );
  const jumpLy = $derived(ly(card.jump_ly));
</script>

{#snippet scoopTag()}
  {#if card.scoopable != null}
    <span class="tag" class:ok={card.scoopable}>
      {card.scoopable ? t('strip-scoopable') : t('strip-not-scoopable')}
    </span>
  {/if}
{/snippet}

{#snippet specialTag()}
  {#if specialKey}
    <span class="tag hot" data-testid="special">{t(specialKey)}</span>
  {/if}
{/snippet}

{#snippet stateTag()}
  {#if pol?.faction_state}
    {#if danger}
      <span class="warn" role="img" aria-label={t('strip-state-danger')}>⚠</span>
    {/if}
    <span class="state" class:danger data-testid="faction-state">{pol.faction_state}</span>
  {/if}
{/snippet}

{#if big}
  <article class="card big" data-testid="system-card">
    <div data-testid="next-card" class="cols">
      <section class="col">
        <h3>{t('strip-col-jump')}</h3>
        <strong class="name">{card.name}</strong>
        <div><span class="label">{t('strip-jump-ly')}</span> <span class="mono">{jumpLy}</span></div>
        {#if remainingJumps != null}
          <div><span class="label">{t('strip-remaining-jumps')}</span> <span class="mono">{remainingJumps}</span></div>
        {/if}
        {#if totalLy != null}
          <div><span class="label">{t('strip-total-ly')}</span> <span class="mono">{ly(totalLy)}</span></div>
        {/if}
        <div>
          <span class="label">{t('strip-star-class')}</span>
          <span class="mono">{starClassLabel(card.star_class)}</span>
          {@render scoopTag()}
        </div>
        {#if card.fuel_after_pct != null}
          <div><span class="label">{t('strip-fuel-after')}</span> <span class="mono">{pct(card.fuel_after_pct / 100)}</span></div>
        {/if}
        {#if card.reachable != null}
          <div class="tag" class:ok={card.reachable} class:hot={!card.reachable}>
            {card.reachable ? t('strip-reachable') : t('strip-unreachable')}
          </div>
        {/if}
      </section>

      <section class="col">
        <h3>{t('strip-col-exploration')}</h3>
        <div><span class="label">{t('strip-discovered')}</span> <span>{discovered}</span></div>
        <div>
          <span class="label">{t('strip-bodies')}</span>
          {#if remoteOk && card.remote.data?.body_count != null}
            <span class="mono">{card.remote.data.body_count}</span>
          {:else}
            <span class="muted">{noData}</span>
          {/if}
        </div>
        <div><span class="label">{t('strip-visits')}</span> <span class="mono">{card.visits ?? '—'}</span></div>
        {@render specialTag()}
      </section>

      <section class="col">
        <h3>{t('strip-col-politics')}</h3>
        {#if pol}
          <div><span class="label">{t('strip-politics-allegiance')}</span> <span>{pol.allegiance ?? '—'}</span></div>
          <div><span class="label">{t('strip-politics-government')}</span> <span>{pol.government ?? '—'}</span></div>
          <div><span class="label">{t('strip-politics-faction')}</span> <span>{pol.controlling_faction ?? '—'}</span></div>
          <div>{@render stateTag()}</div>
        {:else}
          <span class="muted">{noData}</span>
        {/if}
      </section>

      <section class="col">
        <h3>{t('strip-col-system')}</h3>
        {#if pol}
          <div>
            <span class="label">{t('strip-politics-economy')}</span>
            <span>{pol.economy ?? '—'}{pol.second_economy ? ` / ${pol.second_economy}` : ''}</span>
          </div>
          <div><span class="label">{t('strip-politics-security')}</span> <span>{pol.security ?? '—'}</span></div>
          <div><span class="label">{t('strip-politics-population')}</span> <span class="mono">{population ?? '—'}</span></div>
          {#if pol.powers && pol.powers.length > 0}
            <div>
              <span class="label">{t('strip-powerplay')}</span>
              <span>{pol.powers.join(', ')}{pol.powerplay_state ? ` · ${pol.powerplay_state}` : ''}</span>
            </div>
          {/if}
        {:else}
          <span class="muted">{noData}</span>
        {/if}
        {#if cacheAge}
          <div class="muted" data-testid="cache-age">{t('strip-cache-age', { age: cacheAge })}</div>
        {/if}
      </section>
    </div>
  </article>
{:else}
  <article class="card small" data-testid="small-card">
    <strong class="name">{card.name}</strong>
    <div>
      <span class="mono">{starClassLabel(card.star_class)}</span>
      {#if specialKey}
        {@render specialTag()}
      {:else}
        {@render scoopTag()}
      {/if}
    </div>
    <div><span class="label">{t('strip-jump-ly')}</span> <span class="mono">{jumpLy}</span></div>
    <div><span class="label">{t('strip-discovered')}</span> <span>{discovered}</span></div>
    {#if pol}
      <div>{pol.allegiance ?? '—'} · {pol.government ?? '—'}</div>
      <div>{@render stateTag()}</div>
    {:else}
      <span class="muted">{noData}</span>
    {/if}
    {#if cacheAge}
      <div class="muted" data-testid="cache-age">{t('strip-cache-age', { age: cacheAge })}</div>
    {/if}
  </article>
{/if}

<style>
  .card {
    background: var(--hud-surface-alt);
    border: 1px solid var(--hud-border);
    border-radius: 4px;
    padding: 0.75rem;
    min-width: 0;
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1rem;
  }

  @media (max-width: 1599px) {
    .cols {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 899px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }

  .col h3 {
    margin: 0 0 0.2rem;
    font-size: 0.8em;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--hud-secondary);
  }

  .small {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .name {
    font-family: var(--font-mono);
    color: var(--hud-primary);
    font-size: var(--font-body-lg);
  }

  .big .name {
    font-size: var(--font-title-lg);
  }

  .label,
  .muted {
    color: var(--hud-muted);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .tag {
    border: 1px solid var(--hud-border);
    border-radius: 3px;
    padding: 0 0.35rem;
    font-size: 0.85em;
    width: fit-content;
  }

  .tag.ok {
    color: var(--hud-success);
    border-color: var(--hud-success);
  }

  .tag.hot {
    color: var(--hud-warning);
    border-color: var(--hud-warning);
    font-weight: 700;
  }

  .warn {
    color: var(--hud-danger);
    margin-right: 0.25rem;
  }

  .state.danger {
    color: var(--hud-danger);
    font-weight: 700;
  }
</style>
