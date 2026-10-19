export type Badge =
  | { kind: 'scanning' | 'clean' | 'lawless' | 'enemy' | 'unknown' }
  | { kind: 'wanted'; bounty: number };

export type SubsystemKind =
  | 'power_plant' | 'drive' | 'fsd' | 'life_support' | 'distributor'
  | 'sensors' | 'weapons' | 'cargo_hatch' | 'other';

export interface Subsystem {
  kind: SubsystemKind;
  name: string;
  health_pct: number | null;
}

/** Topic `combat.target` (null cuando no hay objetivo). */
export interface TargetTopic {
  ship_key: string | null;
  ship_name: string | null;
  pilot: string | null;
  rank: string | null;
  faction: string | null;
  badge: Badge;
  shield_pct: number | null;
  hull_pct: number | null;
  subsystem: Subsystem | null;
  thargoid: boolean;
  lost_at: string | null;
}

export interface Pips {
  sys: number;
  eng: number;
  wep: number;
}

export type HeatState = 'normal' | 'overheating' | 'heat_damage';
export type ShipAlert = 'hull_low' | 'shields_down' | 'overheating' | 'under_attack' | 'interdiction';

/** Topic `combat.ship`. */
export interface ShipTopic {
  shields_up: boolean | null;
  hull_pct: number | null;
  heat: HeatState;
  overheating_since: string | null;
  overheat_total_s: number;
  pips: Pips | null;
  hardpoints: boolean;
  alerts: ShipAlert[];
}

export type VoucherKind = 'Bounty' | 'CombatBond';

export interface VoucherRow {
  kind: VoucherKind | null;
  faction: string;
  amount: number;
  systems: string[];
  ax: boolean;
  other: boolean;
  count: number;
}

export interface RemoteStation {
  station_name: string;
  system_name: string;
  distance_ly: number;
  distance_to_arrival_ls: number | null;
  max_pad: 'Small' | 'Medium' | 'Large' | null;
  updated_at: string | null;
}

/** Topic `combat.session`. */
export interface SessionTopic {
  /** Último instante del diario (base de tiempo); ausente o época cero sin eventos. */
  as_of?: string | null;
  started_at: string | null;
  kills: number;
  credits: number;
  kills_by_type: Record<string, number>;
  combat_rank: number | null;
  combat_progress: number | null;
  vouchers: VoucherRow[];
  vouchers_total: { bounty: number; combat_bond: number };
  nearest_if: RemoteStation | null;
  if_fee_pct: number;
}

/** Topic `combat.conflict_zone` (null fuera de una zona de conflicto). */
export interface ZoneTopic {
  system: string | null;
  side: string | null;
  kills: number;
  bonds_by_faction: Record<string, number>;
}

/** Topic `combat.ax`. */
export interface AxTopic {
  active: boolean;
  presence: boolean;
  kills_by_type: Record<string, number>;
  bonds: number;
  target: string | null;
}

/** Nombre de facción legible: limpia `$faction_X;` y deja vacío si no hay. */
export function cleanFaction(raw: string | null | undefined): string {
  const s = (raw ?? '').trim();
  const m = s.match(/^\$(.*?);?$/);
  if (!m) return s;
  return m[1].replace(/^faction_/i, '').replaceAll('_', ' ').trim();
}
