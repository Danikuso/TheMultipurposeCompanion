<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
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
    open: boolean;
    timer: ReturnType<typeof setTimeout> | null;
    ctrl: AbortController | null;
  }

  const DEBOUNCE_MS = 300;
  const MIN_CHARS = 3;

  const plan = $derived(topic<PlanTopic>('navigation.plan'));
  const status = $derived(plan?.status ?? 'idle');
  const disabled = $derived(status === 'disabled');
  const computing = $derived(status === 'computing');

  let to = $state<Field>({ text: '', id64: null, items: [], seq: 0, open: false, timer: null, ctrl: null });
  let from = $state<Field>({ text: '', id64: null, items: [], seq: 0, open: false, timer: null, ctrl: null });
  let mode = $state<'neutron' | 'galaxy'>('neutron');
  let efficiency = $state<number | null>(null);
  let defaultEfficiency = $state<number | null>(null);
  let useShip = $state(true);
  let refuelScoopable = $state(true);
  let useInjections = $state(false);

  onMount(async () => {
    try {
      const res = await fetch('/api/config');
      if (!res.ok) return;
      const cfg = (await res.json()) as { default_neutron_efficiency?: unknown };
      const v = cfg.default_neutron_efficiency;
      if (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 100) defaultEfficiency = v;
    } catch {
      // sin ajustes: se usa el texto genérico
    }
  });

  onDestroy(() => {
    for (const f of [to, from]) {
      if (f.timer !== null) clearTimeout(f.timer);
      f.ctrl?.abort();
    }
  });

  const efficiencyValue = $derived.by(() => {
    const raw: unknown = efficiency;
    if (raw === null || raw === undefined || raw === '') return null;
    const n = Number(raw);
    return Number.isInteger(n) && n >= 0 && n <= 100 ? n : NaN;
  });
  const efficiencyInvalid = $derived(Number.isNaN(efficiencyValue));
  const hasDestination = $derived(to.text.trim() !== '' || Boolean(plan?.active?.destination));
  const toNeedsPick = $derived(mode === 'galaxy' && to.text.trim() !== '' && to.id64 == null);
  const fromNeedsPick = $derived(mode === 'galaxy' && from.text.trim() !== '' && from.id64 == null);
  const blocked = $derived(disabled || computing || efficiencyInvalid || !hasDestination || toNeedsPick || fromNeedsPick);

  async function fetchSuggestions(f: Field, seq: number) {
    f.ctrl?.abort();
    const ctrl = new AbortController();
    f.ctrl = ctrl;
    try {
      const res = await fetch(`/api/navigation/autocomplete?q=${encodeURIComponent(f.text.trim())}&page=0`, {
        signal: ctrl.signal,
      });
      if (seq !== f.seq) return;
      if (!res.ok) {
        f.items = [];
        return;
      }
      const body = (await res.json()) as { items?: Suggestion[] };
      if (seq === f.seq) {
        f.items = body.items ?? [];
        f.open = f.items.length > 0;
      }
    } catch {
      if (seq === f.seq) f.items = [];
    }
  }

  function onType(f: Field) {
    f.id64 = null;
    f.seq += 1;
    f.ctrl?.abort();
    if (f.timer !== null) {
      clearTimeout(f.timer);
      f.timer = null;
    }
    if (f.text.trim().length < MIN_CHARS) {
      f.items = [];
      f.open = false;
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
    f.open = false;
  }

  function closeLater(f: Field) {
    // deja que el clic en una sugerencia se procese antes de cerrar la lista
    setTimeout(() => {
      f.open = false;
    }, 150);
  }

  function onKey(ev: KeyboardEvent, f: Field) {
    if (ev.key === 'Escape') f.open = false;
  }

  function submit(ev: Event) {
    ev.preventDefault();
    if (blocked) return;
    const targetDest = to.text.trim() || plan?.active?.destination || '';
    if (!targetDest) return;
    const args: Record<string, unknown> = {
      to: targetDest,
      mode,
      use_ship: useShip,
    };
    if (efficiencyValue !== null) args.efficiency = efficiencyValue;
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

  function recalculate() {
    void action('navigation.recalculate', {}).catch(() => {});
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
        role="combobox"
        aria-expanded={from.open && from.items.length > 0}
        aria-controls="plan-from-list"
        aria-autocomplete="list"
        oninput={() => onType(from)}
        onkeydown={(e) => onKey(e, from)}
        onblur={() => closeLater(from)}
      />
      {#if from.open && from.items.length > 0}
        <ul class="suggestions" id="plan-from-list" role="listbox">
          {#each from.items as s (s.id64)}
            <li role="option" aria-selected="false"><button type="button" data-testid="suggestion-from" onclick={() => pick(from, s)}>{s.name}</button></li>
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
        placeholder={plan?.active?.destination ? `Destino (o ruta activa: ${plan.active.destination})` : ''}
        {disabled}
        bind:value={to.text}
        role="combobox"
        aria-expanded={to.open && to.items.length > 0}
        aria-controls="plan-to-list"
        aria-autocomplete="list"
        oninput={() => onType(to)}
        onkeydown={(e) => onKey(e, to)}
        onblur={() => closeLater(to)}
      />
      {#if to.open && to.items.length > 0}
        <ul class="suggestions" id="plan-to-list" role="listbox">
          {#each to.items as s (s.id64)}
            <li role="option" aria-selected="false"><button type="button" data-testid="suggestion" onclick={() => pick(to, s)}>{s.name}</button></li>
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
          min="0"
          max="100"
          data-testid="plan-efficiency"
          placeholder={defaultEfficiency != null
            ? t('nav-plan-efficiency-default', { value: defaultEfficiency })
            : t('nav-plan-efficiency-hint')}
          aria-invalid={efficiencyInvalid}
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
      <button type="submit" class="primary" data-testid="plan-submit" disabled={blocked}>
        {t('nav-plan-submit')}
      </button>
    </div>
  </form>

  {#if efficiencyInvalid}
    <p class="error" data-testid="plan-efficiency-error">{t('nav-plan-efficiency-invalid')}</p>
  {/if}
  {#if toNeedsPick || fromNeedsPick}
    <p class="note" data-testid="plan-pick-hint">{t('nav-plan-pick-destination')}</p>
  {/if}
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
      <div class="active-actions">
        <button type="button" class="btn-recalc" data-testid="plan-recalc" onclick={recalculate}>
          {t('strip-recalculate')}
        </button>
        <button type="button" data-testid="plan-clear" onclick={clear}>{t('nav-plan-clear')}</button>
      </div>
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
          {#each plan.result.hops as hop, i (`${hop.id64}-${i}`)}
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
    width: 9rem;
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
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.6rem;
  }

  .active-actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-shrink: 0;
  }

  .btn-recalc {
    background: rgba(0, 212, 178, 0.15);
    border: 1px solid rgba(0, 212, 178, 0.4);
    color: #00d4b2;
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 11px;
    cursor: pointer;
    font-weight: 600;
  }

  .btn-recalc:hover {
    background: rgba(0, 212, 178, 0.25);
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
