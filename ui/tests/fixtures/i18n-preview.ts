// Visual review with actual panels and synthetic state; no live session or backend.
import '../../src/styles/base.css';
import { mount } from 'svelte';
import { setLocale } from '../../src/lib/i18n';
import { live } from '../../src/lib/ws.svelte';
import CmdrPanel from '../../src/modules/dashboard/CmdrPanel.svelte';
import TargetPanel from '../../src/modules/combat/TargetPanel.svelte';
import ExobiologyPanel from '../../src/modules/exobiology/ExobiologyPanel.svelte';
import MiningSessionPanel from '../../src/modules/mining/MiningSessionPanel.svelte';
import ShipPanel from '../../src/modules/dashboard/ShipPanel.svelte';
import AccountsPanel from '../../src/modules/accounts/AccountsPanel.svelte';
import MiningTargetsPanel from '../../src/modules/mining/MiningTargetsPanel.svelte';

setLocale(new URLSearchParams(location.search).get('lang') ?? 'es');
live.connected = true;
live.topics['core.cmdr'] = {
  name: 'Synthetic', credits: 50_000_000, loan: 0, game_mode: 'Solo',
  ranks: { combat: 5, trade: 4, explore: 2, soldier: 4, exobiologist: 7, cqc: 3, federation: 9, empire: 12 },
  progress: { combat: 42, trade: 42, explore: 42, soldier: 42, exobiologist: 42, cqc: 42, federation: 42, empire: 42 },
};
live.topics['core.ship'] = {
  ship_type: 'python', name: 'Synthetic', ident: 'QA-001', rebuy: 1_000_000, hull_health: 1,
  fuel_main: 24, fuel_capacity_main: 32, fuel_reservoir: 0.5, fuel_capacity_reserve: 0.7,
  cargo_count: 0, cargo_capacity: 64, max_jump_range: 30, unladen_mass: 500,
  modules: [
    { Slot: 'FrameShiftDrive', Item: 'int_hyperdrive_size5_class5' },
    { Slot: 'MainEngines', Item: 'int_engine_size6_class5' },
    { Slot: 'Slot01_Size5', Item: 'int_fuelscoop_size5_class5' },
    { Slot: 'LargeHardpoint1', Item: 'hpt_multicannon_gimbal_size3_class2' },
  ],
};
live.topics['combat.target'] = {
  ship_key: 'sidewinder', ship_name: 'Sidewinder', pilot: 'Synthetic', rank: 'Master', faction: 'Synthetic Faction',
  badge: { kind: 'clean' }, shield_pct: 100, hull_pct: 100,
  subsystem: { kind: 'cargo_hatch', name: 'Cargo Hatch', health_pct: 100 }, thargoid: false, lost_at: null,
};
live.topics['exobiology.body'] = {
  body: 'Synthetic A 1', body_id: 1, system_address: 1, kind: 'Rocky body', kind_key: 'rocky', gravity_g: 0.1,
  temperature_k: 170, atmosphere: 'Carbon dioxide', atmosphere_key: 'carbon-dioxide', bio_signals: 0, first_logged: null, dss: false,
  candidates: [], confirmed_genera: [], completed: 0,
};
live.topics['exobiology.unsold'] = { total: 0, species: 0 };
live.topics['mining.session'] = {
  started_at: null, as_of: new Date().toISOString(), refined: {}, est_credits: 0,
  refinery_bins: [], limpets_left: 0, limpets_launched: {}, cargo: 0, capacity: 100,
};
window.fetch = async () => new Response('{}');
const style = document.createElement('style');
style.textContent = `body { height: auto; overflow: auto; padding: 12px; }
  #preview { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr)); gap: 12px; }
  .qa-panel { min-width: 0; }`;
document.head.append(style);
for (const component of [CmdrPanel, TargetPanel, ExobiologyPanel, MiningSessionPanel, ShipPanel, AccountsPanel, MiningTargetsPanel]) {
  const target = document.createElement('section'); target.className = 'qa-panel';
  document.getElementById('preview')!.append(target); mount(component, { target });
}
