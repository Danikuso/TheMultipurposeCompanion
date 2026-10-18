<script lang="ts">
  import { t } from '../../lib/i18n';
  import { ageShort, credits, ly, pct } from '../../lib/format';
  import { action, topic } from '../../lib/ws.svelte';
  import Panel from '../../shell/Panel.svelte';
  import { cleanFaction, type SessionTopic } from './types';

  const s = $derived(topic<SessionTopic>('combat.session'));
  let now = $state(Date.now());
  let failed = $state(false);

  // Un único setInterval de 30 s (antigüedad del IF y créditos/hora); no corre sin sesión.
  $effect(() => {
    if (!s) return;
    now = Date.now();
    const id = setInterval(() => {
      now = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  const perHour = $derived.by(() => {
    if (!s?.started_at) return null;
    const secs = Math.max(60, (now - Date.parse(s.started_at)) / 1000);
    return Math.round((s.credits / secs) * 3600);
  });
  const rankName = $derived(
    s?.combat_rank != null && s.combat_rank >= 0 && s.combat_rank <= 8
      ? t(`combat-rank-${s.combat_rank}`)
      : null,
  );
  const ifAge = $derived.by(() => {
    const ts = s?.nearest_if?.updated_at ? Date.parse(s.nearest_if.updated_at) : NaN;
    return Number.isNaN(ts) ? null : ageShort(Math.max(0, (now - ts) / 1000));
  });
  const unredeemed = $derived(s ? s.vouchers_total.bounty + s.vouchers_total.combat_bond : 0);

  function fee(amount: number): string {
    return credits(Math.round((amount * (s?.if_fee_pct ?? 25)) / 100));
  }

  async function reset() {
    failed = false;
    try {
      await action('combat.reset_session', {});
    } catch {
      failed = true;
    }
  }
</script>

<Panel titleKey="panel-combat-hunting" span={2}>
  {#if !s}
    <p class="empty">{t('combat-no-data')}</p>
  {:else}
    <div class="row">
      <strong data-testid="session-kills">{t('combat-kills', { count: s.kills })}</strong>
      <span data-testid="session-credits">{t('combat-session-credits', { credits: credits(s.credits) })}</span>
      <span class="mono" data-testid="credits-hour">
        {perHour == null ? '—' : t('combat-credits-hour', { rate: credits(perHour) })}
      </span>
    </div>
    {#if rankName}
      <div class="row">
        <span class="label">{t('combat-rank')}</span>
        <span data-testid="rank-progress">{t('combat-rank-progress', { rank: rankName, pct: pct(s.combat_progress == null ? null : s.combat_progress / 100) })}</span>
      </div>
    {/if}
    <h3>{t('combat-vouchers-title')}</h3>
    {#if s.vouchers.length === 0}
      <p class="empty" data-testid="vouchers-none">{t('combat-bounties-none')}</p>
    {:else}
      <p class="mono" data-testid="vouchers-total">{t('combat-bounties-unclaimed', { count: s.vouchers.reduce((n, v) => n + v.count, 0), credits: credits(unredeemed) })}</p>
      <ul class="vouchers">
        {#each s.vouchers as v, i (i)}
          <li data-testid="voucher-row" data-kind={v.kind ?? 'other'}>
            <span class="kind">{v.kind === 'Bounty' ? t('combat-voucher-bounty') : v.kind === 'CombatBond' ? t('combat-voucher-bond') : t('combat-voucher-other', { count: v.count })}</span>
            {#if !v.other}
              <strong>{cleanFaction(v.faction) || t('combat-faction-unknown')}</strong>
              {#if v.systems.length > 0}<span class="systems">{v.systems.join(', ')}</span>{/if}
            {/if}
            <span class="mono amount">{credits(v.amount)}</span>
            {#if s.nearest_if}
              <span class="mono fee" data-testid="voucher-fee">{t('combat-if-fee', { fee: fee(v.amount) })}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    <div class="if">
      <span class="label">{t('combat-if-nearest')}</span>
      {#if s.nearest_if}
        <span data-testid="if-station">{t('combat-if-station', { station: s.nearest_if.station_name, system: s.nearest_if.system_name, distance: ly(s.nearest_if.distance_ly) })}</span>
        <span class="muted" data-testid="if-age">{ifAge == null ? '—' : t('combat-if-age', { age: ifAge })}</span>
      {:else}
        <span class="muted" data-testid="if-none">{t('combat-no-data')}</span>
      {/if}
    </div>
    <button type="button" data-testid="reset-session" onclick={reset}>{t('combat-reset-session')}</button>
    {#if failed}<p class="err" role="alert">{t('combat-reset-failed')}</p>{/if}
  {/if}
</Panel>

<style>
  .empty, .muted, .label { color: var(--hud-muted); margin: 0; }
  .row { display: flex; flex-wrap: wrap; gap: 0.75rem; justify-content: space-between; margin-bottom: 0.4rem; }
  .mono { font-family: var(--font-mono); }
  h3 { margin: 0.6rem 0 0.3rem; font-size: 0.9rem; color: var(--hud-muted); }
  .vouchers { list-style: none; margin: 0; padding: 0; }
  .vouchers li { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: baseline; margin-bottom: 0.25rem; }
  .vouchers li[data-kind='Bounty'] .kind { color: var(--hud-warning); }
  .vouchers li[data-kind='CombatBond'] .kind { color: var(--hud-ok); }
  .systems, .fee { color: var(--hud-muted); }
  .amount { margin-left: auto; }
  .if { display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 0.6rem 0; }
  .err { color: var(--hud-danger); }
</style>
