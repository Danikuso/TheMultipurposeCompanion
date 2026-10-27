<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, formatShipName, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import ManufacturerIcon from '../../lib/icons/ManufacturerIcon.svelte';
  import type { SubsystemKind, TargetTopic } from './types';

  const LOST_TTL_MS = 10_000;
  const ICONS: Record<SubsystemKind, string> = {
    power_plant: '⚡', drive: '➤', fsd: '◎', life_support: '✚', distributor: '⌁',
    sensors: '◉', weapons: '✦', cargo_hatch: '▤', other: '◇',
  };
  const KEYS: Record<SubsystemKind, string> = {
    power_plant: 'powerplant', drive: 'drive', fsd: 'fsd', life_support: 'lifesupport',
    distributor: 'distributor', sensors: 'sensors', weapons: 'weapons', cargo_hatch: 'cargohatch', other: 'other',
  };

  const tgt = $derived(topic<TargetTopic | null>('combat.target'));
  const lostAt = $derived(tgt?.lost_at ?? null);
  let expiredFor = $state<string | null>(null);

  // Respaldo: setTimeout único de 10 s desde la RECEPCIÓN de un nuevo `lost_at`
  // (nunca se compara el reloj del diario con el del cliente). Se cancela al
  // cambiar de objetivo o al destruir el panel; el backend también vacía el topic.
  $effect(() => {
    if (lostAt == null) return;
    const id = setTimeout(() => {
      expiredFor = lostAt;
    }, LOST_TTL_MS);
    return () => clearTimeout(id);
  });

  const visible = $derived(tgt != null && !(lostAt != null && expiredFor === lostAt));
  const badgeKind = $derived(tgt?.badge.kind ?? 'unknown');
  const badgeText = $derived.by(() => {
    if (!tgt) return '';
    const b = tgt.badge;
    if (b.kind === 'wanted') return t('combat-badge-wanted', { reward: credits(b.bounty) });
    if (b.kind === 'clean') return t('combat-badge-clean');
    if (b.kind === 'scanning') return t('combat-scanning');
    return t(`combat-badge-${b.kind}`);
  });
</script>

<Panel titleKey="combat-target" span={1}>
  {#if !visible || !tgt}
    <p class="empty" data-testid="empty">{t('combat-no-target')}</p>
  {:else}
    <div class="head">
      <div>
        <div class="target-title-row">
          <strong class="ship">{formatShipName(tgt.ship_key, tgt.ship_name)}</strong>
          <ManufacturerIcon shipType={tgt.ship_name ?? tgt.ship_key} size={16} />
        </div>
        {#if tgt.pilot}<div class="dim">{tgt.pilot}{tgt.rank ? ` · ${tgt.rank}` : ''}</div>{/if}
        {#if tgt.faction}<div class="dim">{t('combat-target-faction', { faction: tgt.faction })}</div>{/if}
      </div>
      <div aria-live="polite">
        {#if badgeText}
          <span class="badge" data-testid="target-badge" data-kind={badgeKind}>{badgeText}</span>
        {/if}
      </div>
    </div>
    <div class="bars">
      <div class="bar-row">
        <span class="dim">{t('combat-shields')}</span>
        <div class="bar" role="progressbar" aria-label={t('combat-shields')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={tgt.shield_pct ?? 0}><i class="shield" style:width="{tgt.shield_pct ?? 0}%"></i></div>
        <span class="mono" data-testid="shield-value">{pct(tgt.shield_pct)}</span>
      </div>
      <div class="bar-row">
        <span class="dim">{t('combat-hull')}</span>
        <div class="bar" role="progressbar" aria-label={t('combat-hull')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={tgt.hull_pct ?? 0}><i class="hull" style:width="{tgt.hull_pct ?? 0}%"></i></div>
        <span class="mono" data-testid="hull-value">{pct(tgt.hull_pct)}</span>
      </div>
    </div>
    {#if tgt.subsystem}
      <div class="sub" data-testid="subsystem">
        <span class="icon" aria-hidden="true">{ICONS[tgt.subsystem.kind] ?? ICONS.other}</span>
        <div class="sub-body">
          <div class="dim">{t('combat-subsystem')}</div>
          <strong class="sub-name">{t(`combat-subsystem-${KEYS[tgt.subsystem.kind] ?? 'other'}`)}</strong>
          <div class="bar" role="progressbar" aria-label={t('combat-subsystem')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={tgt.subsystem.health_pct ?? 0}><i class="subbar" style:width="{tgt.subsystem.health_pct ?? 0}%"></i></div>
        </div>
        <span class="mono">{pct(tgt.subsystem.health_pct)}</span>
      </div>
    {/if}
  {/if}
</Panel>

<style>
  .empty {
    color: var(--hud-muted);
    margin: 0;
    font-style: italic;
    padding: 1rem 0;
  }
  .dim {
    color: var(--hud-muted);
    font-size: 0.85rem;
  }
  .mono {
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 0.85rem;
    padding-bottom: 0.6rem;
    border-bottom: 1px solid var(--hud-border-subtle);
  }
  .ship {
    font-size: 1.15rem;
    color: var(--hud-primary);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .target-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .badge {
    display: inline-block;
    border: 1px solid currentColor;
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.65rem;
    font-weight: 700;
    font-size: 0.82rem;
    letter-spacing: 0.06em;
    white-space: nowrap;
    text-transform: uppercase;
    backdrop-filter: blur(4px);
  }
  .badge[data-kind='wanted'] {
    color: var(--hud-danger);
    background: rgba(255, 68, 68, 0.12);
    box-shadow: 0 0 10px rgba(255, 68, 68, 0.25);
  }
  .badge[data-kind='clean'] {
    color: var(--hud-ok);
    background: rgba(68, 255, 136, 0.12);
    box-shadow: 0 0 10px rgba(68, 255, 136, 0.2);
  }
  .badge[data-kind='scanning'], .badge[data-kind='unknown'] {
    color: var(--hud-muted);
    background: rgba(255, 255, 255, 0.05);
  }
  .badge[data-kind='lawless'], .badge[data-kind='enemy'] {
    color: var(--hud-warning);
    background: rgba(255, 170, 0, 0.12);
    box-shadow: 0 0 10px rgba(255, 170, 0, 0.2);
  }
  .bars {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 0.8rem;
  }
  .bar-row {
    display: grid;
    grid-template-columns: 5.2rem 1fr 3.5rem;
    align-items: center;
    gap: 0.75rem;
  }
  .bar-row .dim {
    text-transform: uppercase;
    font-size: 0.78rem;
    letter-spacing: 0.04em;
  }
  .bar {
    height: 0.65rem;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--hud-border-subtle);
    border-radius: 2px;
    overflow: hidden;
    position: relative;
  }
  .bar i {
    display: block;
    height: 100%;
    transition: width var(--transition-fast);
  }
  .shield {
    background: linear-gradient(90deg, #0088aa, var(--hud-cyan));
    box-shadow: 0 0 8px rgba(0, 217, 255, 0.45);
  }
  .hull {
    background: linear-gradient(90deg, #b34700, var(--hud-primary));
    box-shadow: 0 0 8px rgba(255, 119, 0, 0.45);
  }
  .subbar {
    background: linear-gradient(90deg, #b37400, var(--hud-warning));
    box-shadow: 0 0 6px rgba(255, 170, 0, 0.45);
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    margin-top: 0.6rem;
    padding: 0.65rem 0.85rem;
    background: rgba(255, 170, 0, 0.04);
    border: 1px solid rgba(255, 170, 0, 0.28);
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 12px rgba(255, 170, 0, 0.03);
  }
  .icon {
    font-size: 1.6em;
    color: var(--hud-warning);
    filter: drop-shadow(0 0 4px rgba(255, 170, 0, 0.4));
  }
  .sub-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .sub-name {
    font-size: 0.95rem;
    color: var(--hud-text);
    font-weight: 600;
  }
</style>
