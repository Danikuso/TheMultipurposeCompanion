export type WeaponMode = 'Fixed' | 'Gimbal' | 'Turret';
export type PadSizeLabel = 'Small' | 'Medium' | 'Large';
export type ModuleCategory =
  | 'core_internal'
  | 'optional_internal'
  | 'hardpoint'
  | 'utility'
  | 'armour';

export interface OutfittingShipEntry {
  name: string;
  symbol: string;
  pad: 'S' | 'M' | 'L';
}

export interface OutfittingModuleEntry {
  name: string;
  category: ModuleCategory;
  has_weapon_mode: boolean;
  is_armour: boolean;
}

export interface PermitSystemEntry {
  system: string;
  permit_key: string;
  requires_elite: boolean;
}

export interface OutfittingCommanderContext {
  current_system: string;
  current_ship_pad: PadSizeLabel | null;
  pledged_power: string | null;
  has_elite_rank: boolean;
  credits: number | null;
  rebuy: number | null;
}

export interface OutfittingCatalogTopic {
  ships: OutfittingShipEntry[];
  modules: OutfittingModuleEntry[];
  classes: number[];
  ratings: string[];
  permit_systems: PermitSystemEntry[];
  commander: OutfittingCommanderContext;
}

export interface OutfittingModuleQuery {
  name: string;
  class?: number | null;
  rating?: string | null;
  weapon_mode?: WeaponMode | null;
  armour_ship?: string | null;
}

export interface StockedItemPrice {
  label: string;
  price_cr: number | null;
}

export type AffordabilityState = 'ok' | 'tight_rebuy' | 'insufficient' | 'unknown';

export interface OutfittingStationRow {
  station_name: string;
  system_name: string;
  station_type: string | null;
  market_id: number | null;
  distance_ly: number;
  distance_to_arrival_ls: number | null;
  max_pad: PadSizeLabel | null;
  is_planetary: boolean;
  is_carrier: boolean;
  is_stronghold_carrier: boolean;
  controlling_power: string | null;
  discount_tag: string | null;
  updated_at: string | null;
  item_prices: StockedItemPrice[];
  total_price_cr: number | null;
  permit_key: string | null;
  permit_requires_elite: boolean;
  affordability: AffordabilityState;
}

export interface OutfittingFallbackRow {
  item_label: string;
  nearest_station: OutfittingStationRow | null;
}

export type OutfittingSearchStatus =
  | 'idle'
  | 'loading'
  | 'ok'
  | 'fallback'
  | 'no_data'
  | 'offline'
  | 'unknown_origin'
  | 'missing_origin'
  | 'error';

export interface OutfittingSearchTopic {
  status: OutfittingSearchStatus;
  pad_mode: 'auto' | 'S' | 'M' | 'L';
  exclude_permits: boolean;
  hidden_permit_count: number;
  params: {
    reference_system: string;
    ship: string | null;
    modules: OutfittingModuleQuery[];
  } | null;
  stations: OutfittingStationRow[];
  fallback: OutfittingFallbackRow[];
  fetched_at: string | null;
  source: string | null;
  as_of: string | null;
}
