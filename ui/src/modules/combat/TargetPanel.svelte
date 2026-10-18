<script lang="ts">
  import { t } from '../../lib/i18n';
  import { credits, pct } from '../../lib/format';
  import { topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
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

  // Un único setTimeout a lost_at + 10 s; el backend también vacía el topic.
  $effect(() => {
    if (lostAt == null) return;
    const delay = Math.max(0, Date.parse(lostAt) + LOST_TTL_MS - Date.now());
    const id = setTimeout(() => {
      expiredFor = lostAt;
    }, delay);
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
    return '';
  });
</script>

<Panel titleKey="combat-target" span={1}>
  {#if !visible || !tgt}
    <p class="empty" data-testid="empty">{t('combat-no-target')}</p>
  {:else}
    <div class="head">
      <div>
        <strong class="ship">{tgt.ship_name ?? '—'}</strong>
        {#if tgt.pilot}<div class="dim">{tgt.pilot}{tgt.rank ? ` · ${tgt.rank}` : ''}</div>{/if}
        {#if tgt.faction}<div class="dim">{t('combat-target-faction', { faction: tgt.faction })}</div>{/if}
      </div>
      {#if badgeText}
        <span class="badge" data-testid="target-badge" data-kind={badgeKind}>{badgeText}</span>
      {/if}
    </div>
    <div class="bars">
      <div class="bar-row">
        <span class="dim">{t('combat-shields')}</span>
        <div class="bar"><i class="shield" style:width="{tgt.shield_pct ?? 0}%"></i></div>
        <span class="mono" data-testid="shield-value">{pct(tgt.shield_pct)}</span>
      </div>
      <div class="bar-row">
        <span class="dim">{t('combat-hull')}</span>
        <div class="bar"><i class="hull" style:width="{tgt.hull_pct ?? 0}%"></i></div>
        <span class="mono" data-testid="hull-value">{pct(tgt.hull_pct)}</span>
      </div>
    </div>
    {#if tgt.subsystem}
      <div class="sub" data-testid="subsystem">
        <span class="icon" aria-hidden="true">{ICONS[tgt.subsystem.kind] ?? ICONS.other}</span>
        <div class="sub-body">
          <div class="dim">{t('combat-subsystem')}</div>
          <strong class="sub-name">{t(`combat-subsystem-${KEYS[tgt.subsystem.kind] ?? 'other'}`)}</strong>
          <div class="bar"><i class="subbar" style:width="{tgt.subsystem.health_pct ?? 0}%"></i></div>
        </div>
        <span class="mono">{pct(tgt.subsystem.health_pct)}</span>
      </div>
    {/if}
  {/if}
</Panel>

<style>
  .empty { color: var(--hud-muted); margin: 0; }
  .dim { color: var(--hud-muted); }
  .mono { font-family: var(--font-mono); }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; margin-bottom: 0.6rem; }
  .ship { font-size: var(--font-title-lg); color: var(--hud-primary); }
  .badge { border: 2px solid currentColor; padding: 0.2rem 0.7rem; font-weight: bold; font-size: 1.2em; white-space: nowrap; }
  .badge[data-kind='wanted'] { color: var(--hud-danger); }
  .badge[data-kind='clean'] { color: var(--hud-ok); }
  .badge[data-kind='scanning'] { color: var(--hud-muted); }
  .bar-row { display: grid; grid-template-columns: 5rem 1fr 4rem; align-items: center; gap: 0.5rem; margin-bottom: 0.3rem; }
  .bar { height: 0.6rem; border: 1px solid var(--hud-border); }
  .bar i { display: block; height: 100%; }
  .shield { background: var(--hud-secondary); }
  .hull { background: var(--hud-primary); }
  .subbar { background: var(--hud-warning); }
  .sub { display: flex; align-items: center; gap: 0.75rem; margin-top: 0.7rem; padding: 0.6rem; border: 1px solid var(--hud-warning); }
  .icon { font-size: 1.8em; color: var(--hud-warning); }
  .sub-body { flex: 1; }
  .sub-name { font-size: var(--font-title-lg); }
</style>
