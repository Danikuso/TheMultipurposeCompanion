<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from '../../lib/i18n';
  import { ly } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import type { PlanTopic } from './types';

  interface Suggestion {
    name: string;
    id64: number;
  }

  interface Field {
    text: string;
    id64: number | null;
    items: Suggestion[];
    seq: number;
    timer: ReturnType<typeof setTimeout> | null;
  }

  const DEBOUNCE_MS = 300;
  const MIN_CHARS = 3;

  const plan = $derived(topic<PlanTopic>('navigation.plan'));
  const status = $derived(plan?.status ?? 'idle');
  const disabled = $derived(status === 'disabled');
  const computing = $derived(status === 'computing');

  let to = $state<Field>({ text: '', id64: null, items: [], seq: 0, timer: null });
  let from = $state<Field>({ text: '', id64: null, items: [], seq: 0, timer: null });
  let mode = $state<'neutron' | 'galaxy'>('neutron');
  let efficiency = $state(60);
  let useShip = $state(true);
  let refuelScoopable = $state(true);
  let useInjections = $state(false);

  onDestroy(() => {
    for (const f of [to, from]) {
      if (f.timer !== null) clearTimeout(f.timer);
    }
  });

  async function fetchSuggestions(f: Field, seq: number) {
    try {
      const res = await fetch(`/api/navigation/autocomplete?q=${encodeURIComponent(f.text.trim())}&page=0`);
      if (!res.ok) return;
      const body = (await res.json()) as { items?: Suggestion[] };
      if (seq === f.seq) f.items = body.items ?? [];
    } catch {
      if (seq === f.seq) f.items = [];
    }
  }

  function onType(f: Field) {
    f.id64 = null;
    f.seq += 1;
    if (f.timer !== null) {
      clearTimeout(f.timer);
      f.timer = null;
    }
    if (f.text.trim().length < MIN_CHARS) {
      f.items = [];
      return;
    }
    const seq = f.seq;
    f.timer = setTimeout(() => {
      f.timer = null;
      void fetchSuggestions(f, seq);
    }, DEBOUNCE_MS);
  }

  function pick(f: Field, s: Suggestion) {
    f.seq += 1;
    f.text = s.name;
    f.id64 = s.id64;
    f.items = [];
  }

  function submit(ev: Event) {
    ev.preventDefault();
    const args: Record<string, unknown> = {
      to: to.text.trim(),
      mode,
      efficiency: Math.min(100, Math.max(1, Math.round(Number(efficiency) || 60))),
      use_ship: useShip,
    };
    if (to.id64 != null) args.to_id64 = to.id64;
    if (from.text.trim()) {
      args.from = from.text.trim();
      if (from.id64 != null) args.from_id64 = from.id64;
    }
    if (mode === 'galaxy') {
      args.refuel_scoopable = refuelScoopable;
      args.use_injections = useInjections;
    }
    void action('navigation.plan', args).catch(() => {});
  }

  function accept() {
    void action('navigation.accept-plan', {}).catch(() => {});
  }

  function clear() {
    void action('navigation.clear-plan', {}).catch(() => {});
  }

  function refuelLabel(v: boolean | null): string {
    if (v == null) return '—';
    return v ? t('nav-plan-refuel-yes') : t('nav-plan-refuel-no');
  }

  const hidden = $derived(plan?.result ? Math.max(0, plan.result.hops_total - plan.result.hops.length) : 0);
</script>

<Panel titleKey="panel-planner" span={2}>
  <form class="form" onsubmit={submit}>
    <div class="field">
      <label for="plan-from">{t('nav-plan-from')}</label>
      <input
        id="plan-from"
        data-testid="plan-from"
        type="text"
        autocomplete="off"
        placeholder={t('nav-plan-from-hint')}
        {disabled}
        bind:value={from.text}
        oninput={() => onType(from)}
      />
      {#if from.items.length > 0}
        <ul class="suggestions">
          {#each from.items as s (s.id64)}
            <li><button type="button" data-testid="suggestion-from" onclick={() => pick(from, s)}>{s.name}</button></li>
          {/each}
        </ul>
      {/if}
    </div>
    <div class="field">
      <label for="plan-to">{t('nav-plan-to')}</label>
      <input
        id="plan-to"
        data-testid="plan-to"
        type="text"
        autocomplete="off"
        {disabled}
        bind:value={to.text}
        oninput={() => onType(to)}
      />
      {#if to.items.length > 0}
        <ul class="suggestions">
          {#each to.items as s (s.id64)}
            <li><button type="button" data-testid="suggestion" onclick={() => pick(to, s)}>{s.name}</button></li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="options">
      <label class="check">
        <input type="radio" name="plan-mode" value="neutron" data-testid="mode-neutron" {disabled} bind:group={mode} />
        {t('nav-plan-mode-neutron')}
      </label>
      <label class="check">
        <input type="radio" name="plan-mode" value="galaxy" data-testid="mode-galaxy" {disabled} bind:group={mode} />
        {t('nav-plan-mode-galaxy')}
      </label>
      <label class="check">
        {t('nav-plan-efficiency')}
        <input
          class="num"
          type="number"
          min="1"
          max="100"
          data-testid="plan-efficiency"
          {disabled}
          bind:value={efficiency}
        />
      </label>
      <label class="check">
        <input type="checkbox" data-testid="plan-use-ship" {disabled} bind:checked={useShip} />
        {t('nav-plan-use-ship')}
      </label>
      {#if mode === 'galaxy'}
        <label class="check">
          <input type="checkbox" data-testid="plan-refuel" {disabled} bind:checked={refuelScoopable} />
          {t('nav-plan-refuel-scoopable')}
        </label>
        <label class="check">
          <input type="checkbox" data-testid="plan-injections" {disabled} bind:checked={useInjections} />
          {t('nav-plan-injections')}
        </label>
      {/if}
    </div>

    <div class="actions">
      <button type="submit" class="primary" data-testid="plan-submit" disabled={disabled || computing}>
        {t('nav-plan-submit')}
      </button>
    </div>
  </form>

  {#if computing}
    <p class="note" data-testid="plan-computing">{t('nav-plan-computing')}</p>
  {/if}
  {#if plan?.error_key && (status === 'failed' || status === 'disabled')}
    <p class="error" data-testid="plan-error" role="alert">{t(plan.error_key)}</p>
  {/if}

  {#if plan?.active && status !== 'ready'}
    <div class="active" data-testid="plan-active">
      <span>
        {t('nav-plan-active', {
          destination: plan.active.destination,
          current: plan.active.current_index,
          total: plan.active.total,
        })}
      </span>
      <button type="button" data-testid="plan-clear" onclick={clear}>{t('nav-plan-clear')}</button>
    </div>
  {/if}

  {#if status === 'ready' && plan?.result}
    <div class="result">
      <div class="summary">
        <span>{t('nav-plan-total')}: <strong class="mono" data-testid="plan-total">{ly(plan.result.total_ly)}</strong></span>
        <span>{t('nav-plan-hops')}: <strong class="mono">{plan.result.hops_total}</strong></span>
      </div>
      <table>
        <thead>
          <tr>
            <th>{t('strip-col-system')}</th>
            <th class="r">{t('nav-plan-distance')}</th>
            <th>{t('strip-star-class')}</th>
            <th>{t('nav-plan-refuel')}</th>
          </tr>
        </thead>
        <tbody>
          {#each plan.result.hops as hop (hop.id64)}
            <tr data-testid="hop">
              <td>
                {hop.name}
                {#if hop.neutron}<span class="badge">{t('strip-neutron')}</span>{/if}
              </td>
              <td class="r mono">{ly(hop.distance_ly)}</td>
              <td>{hop.star_class ?? '—'}</td>
              <td>{refuelLabel(hop.refuel)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if hidden > 0}
        <p class="note" data-testid="plan-more">{t('nav-plan-more', { count: hidden })}</p>
      {/if}
      <div class="actions">
        <button type="button" class="primary" data-testid="plan-accept" onclick={accept}>
          {t('nav-plan-accept')}
        </button>
      </div>
    </div>
  {/if}
</Panel>

<style>
  .form {
    display: grid;
    gap: 0.6rem;
  }

  .field {
    position: relative;
    display: grid;
    gap: 0.2rem;
  }

  label {
    color: var(--hud-muted);
  }

  input[type='text'],
  input.num {
    background: transparent;
    color: var(--hud-primary);
    border: 1px solid var(--hud-muted);
    border-radius: 3px;
    padding: 0.25rem 0.4rem;
    font: inherit;
  }

  input.num {
    width: 4.5rem;
    margin-left: 0.4rem;
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1rem;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .suggestions {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--hud-muted);
    border-radius: 3px;
    max-height: 12rem;
    overflow-y: auto;
  }

  .suggestions button {
    width: 100%;
    text-align: left;
    background: transparent;
    color: var(--hud-primary);
    border: 0;
    padding: 0.25rem 0.4rem;
    font: inherit;
    cursor: pointer;
  }

  .suggestions button:hover,
  .suggestions button:focus-visible {
    background: color-mix(in srgb, var(--hud-primary) 20%, transparent);
  }

  button {
    font: inherit;
    cursor: pointer;
  }

  button.primary {
    background: transparent;
    color: var(--hud-primary);
    border: 1px solid var(--hud-primary);
    border-radius: 3px;
    padding: 0.3rem 0.9rem;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .actions {
    display: flex;
    gap: 0.6rem;
    margin-top: 0.4rem;
  }

  .note {
    color: var(--hud-muted);
    margin: 0.5rem 0 0;
  }

  .error {
    color: var(--hud-danger, var(--hud-secondary));
    margin: 0.5rem 0 0;
  }

  .active,
  .summary {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.75rem;
    margin-top: 0.6rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.5rem;
  }

  th {
    color: var(--hud-muted);
    font-weight: normal;
    text-align: left;
  }

  th.r,
  td.r {
    text-align: right;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .badge {
    color: var(--hud-secondary);
    border: 1px solid var(--hud-secondary);
    border-radius: 3px;
    padding: 0 0.3rem;
    margin-left: 0.3rem;
    font-size: 0.85em;
  }
</style>
