<script lang="ts">
  import { t } from '../../lib/i18n';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';

  interface DestinationInfo {
    System?: number;
    Body?: number;
    Name?: string;
    Name_Localised?: string;
    system?: number;
    body?: number;
    name?: string;
    name_localised?: string;
  }

  interface StatusTopic {
    Flags?: number;
    Flags2?: number;
    Pips?: [number, number, number] | null;
    flags?: number;
    flags2?: number;
    pips?: [number, number, number] | null;
    Destination?: DestinationInfo | null;
    destination?: DestinationInfo | null;
  }

  const FLAG_LANDING_GEAR = 1 << 2;
  const FLAG_SHIELDS_UP = 1 << 3;
  const FLAG_FLIGHT_ASSIST_OFF = 1 << 5;
  const FLAG_HARDPOINTS = 1 << 6;
  const FLAG_LIGHTS = 1 << 8;
  const FLAG_CARGO_SCOOP = 1 << 9;
  const FLAG_SILENT_RUNNING = 1 << 10;
  const FLAG_MASS_LOCKED = 1 << 16;
  const FLAG_IN_DANGER = 1 << 22;
  const FLAG_NIGHT_VISION = 1 << 28;
  const FLAG2_SCO_ACTIVE = 1 << 20;

  const status = $derived(topic<StatusTopic>('core.status'));
  const flags = $derived(status?.Flags ?? status?.flags ?? 0);
  const flags2 = $derived(status?.Flags2 ?? status?.flags2 ?? 0);
  const pips = $derived(status?.Pips ?? status?.pips ?? null);

  const destinationName = $derived.by(() => {
    const dest = status?.Destination ?? status?.destination;
    if (!dest) return null;
    return dest.Name_Localised ?? dest.name_localised ?? dest.Name ?? dest.name ?? null;
  });

  const chips = $derived([
    { key: 'flag-shields', active: (flags & FLAG_SHIELDS_UP) !== 0, danger: false, warning: false },
    { key: 'flag-hardpoints', active: (flags & FLAG_HARDPOINTS) !== 0, danger: false, warning: false },
    { key: 'flag-landing-gear', active: (flags & FLAG_LANDING_GEAR) !== 0, danger: false, warning: false },
    { key: 'flag-mass-locked', active: (flags & FLAG_MASS_LOCKED) !== 0, danger: false, warning: false },
    { key: 'flag-silent-running', active: (flags & FLAG_SILENT_RUNNING) !== 0, danger: false, warning: false },
    { key: 'flag-cargo-scoop', active: (flags & FLAG_CARGO_SCOOP) !== 0, danger: false, warning: false },
    { key: 'flag-sco', active: (flags2 & FLAG2_SCO_ACTIVE) !== 0, danger: false, warning: false },
    { key: 'flag-lights', active: (flags & FLAG_LIGHTS) !== 0, danger: false, warning: false },
    { key: 'flag-night-vision', active: (flags & FLAG_NIGHT_VISION) !== 0, danger: false, warning: false },
    { key: 'flag-fa-off', active: (flags & FLAG_FLIGHT_ASSIST_OFF) !== 0, danger: false, warning: true },
    { key: 'flag-in-danger', active: (flags & FLAG_IN_DANGER) !== 0, danger: true, warning: false },
  ]);
</script>

<Panel titleKey="panel-status">
  {#if !status}
    <p class="empty">{t('empty-no-data')}</p>
  {:else}
    {#if destinationName}
      <div class="destination-row">
        <span class="dest-label">{t('field-target-destination')}:</span>
        <strong class="dest-val" data-testid="target-destination">{destinationName}</strong>
      </div>
    {/if}

    <div class="chips-wrap">
      {#each chips as chip (chip.key)}
        <span
          class="status-chip"
          class:active={chip.active}
          class:chip-danger={chip.danger && chip.active}
          class:chip-warning={chip.warning && chip.active}
        >
          {t(chip.key)}
        </span>
      {/each}
    </div>

    {#if pips}
      <div class="pips-grid">
        {#each ['SYS', 'ENG', 'WEP'] as label, idx (label)}
          {@const pctVal = Math.min(100, Math.round(((pips[idx] ?? 0) / 8) * 100))}
          <div class="pip-row">
            <span class="pip-label">{label}</span>
            <div class="pip-track">
              <div class="pip-fill" style="width: {pctVal}%"></div>
            </div>
            <span class="pip-val">{((pips[idx] ?? 0) / 2).toFixed(1)}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
  }

  .destination-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    margin-bottom: 0.6rem;
    background: var(--hud-surface-card);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 4px;
    font-size: 0.78rem;
  }

  .dest-label {
    color: var(--hud-muted);
    font-size: 0.72rem;
    text-transform: uppercase;
  }

  .dest-val {
    color: var(--hud-primary);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chips-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 0.65rem;
  }

  .status-chip {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    border: 1px solid var(--hud-border);
    color: var(--hud-muted);
    background: var(--hud-bg);
  }

  .status-chip.active {
    border-color: var(--hud-primary);
    background: var(--hud-primary-dim);
    color: var(--hud-text);
    font-weight: 600;
  }

  .status-chip.chip-warning {
    border-color: #f59e0b;
    background: rgba(245, 158, 11, 0.15);
    color: #fcd34d;
  }

  .status-chip.chip-danger {
    border-color: var(--hud-danger);
    background: rgba(239, 68, 68, 0.2);
    color: #fca5a5;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.65; }
  }

  .pips-grid {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .pip-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.78rem;
  }

  .pip-label {
    width: 2.2rem;
    font-family: var(--font-mono);
    color: var(--hud-muted);
  }

  .pip-track {
    flex: 1;
    height: 6px;
    background: var(--hud-bg);
    border: 1px solid var(--hud-border);
    border-radius: 2px;
    overflow: hidden;
  }

  .pip-fill {
    height: 100%;
    background: var(--hud-secondary);
    transition: width 150ms ease;
  }

  .pip-val {
    width: 2rem;
    text-align: right;
    font-family: var(--font-mono);
  }
</style>
