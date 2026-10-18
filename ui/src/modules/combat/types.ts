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
