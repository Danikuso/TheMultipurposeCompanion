export type CrimeType =
  | 'assault'
  | 'murder'
  | 'piracy'
  | 'interdiction'
  | 'reckless_weapons_discharge'
  | 'illegal_cargo'
  | 'disobey_police'
  | 'docking_major_blocking_avenue'
  | 'docking_minor_blocking_avenue'
  | 'docking_major_tresspass'
  | 'docking_minor_tresspass'
  | 'fire_in_no_fire_zone'
  | 'fire_in_station'
  | 'collided_at_speed_in_station'
  | string;

export interface FactionFineBounty {
  faction: string;
  system?: string;
  fines: number;
  bounties: number;
}

export interface StolenCargoItem {
  name: string;
  name_localised?: string;
  count: number;
  stolen: number;
  is_mission: boolean;
}

export interface NearbyFacility {
  station_name: string;
  system_name: string;
  distance_ly: number;
  pad_size: string;
  is_planetary: boolean;
}

export type SystemSecurityLevel = 'high' | 'medium' | 'low' | 'anarchy' | 'unknown';

export interface CrimeStatusTopic {
  legal_state: string;
  notoriety: number;
  notoriety_decay_seconds: number;
  total_fines: number;
  total_bounties: number;
  factions: FactionFineBounty[];
  stolen_items: StolenCargoItem[];
  nearest_interstellar_factor: NearbyFacility | null;
  nearest_black_market: NearbyFacility | null;
  system_security: SystemSecurityLevel;
  police_response_estimate_seconds: number | null;
}
