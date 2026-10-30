export type PadSize = 'Small' | 'Medium' | 'Large';

export interface MiningTargetQuery {
  reference_system: string;
  reference_coords?: [number, number, number] | null;
  commodity: string;
  min_hotspots: number;
  require_res_combo: boolean;
  populated_only: boolean;
  min_pad: PadSize;
}

export interface MiningSellStation {
  system_name: string;
  station_name: string;
  station_type?: string | null;
  distance_from_ring_ly: number;
  distance_from_ref_ly: number;
  arrival_distance_ls?: number | null;
  pad: PadSize;
  price_cr: number;
  demand: number;
  demand_tax_risk: boolean;
  updated_at: string;
}

export interface MiningRingTarget {
  system_name: string;
  distance_ly: number;
  body_name: string;
  ring_name: string;
  ring_type: string;
  arrival_distance_ls?: number | null;
  hotspot_count: number;
  res_intensity?: string | null;
  res_distance_km?: number | null;
  comment?: string | null;
  other_hotspots: string[];
  nearest_best_sell?: MiningSellStation | null;
}

export interface MiningLoopPlan {
  origin_system: string;
  ring_target: MiningRingTarget;
  sell_station: MiningSellStation;
  total_distance_ly: number;
}

export interface StockedItemPrice {
  label: string;
  price_cr?: number | null;
}

export interface MiningMultiSellHit {
  station_name: string;
  system_name: string;
  distance_ly: number;
  pad: PadSize;
  item_prices: StockedItemPrice[];
  total_revenue_cr: number;
  updated_at?: string | null;
}

export interface CargoMineralItem {
  name: string;
  count: number;
}

export interface MiningTargetsTopic {
  status: 'idle' | 'loading' | 'ok' | 'empty' | 'offline';
  commodity: string;
  query: MiningTargetQuery;
  targets: MiningRingTarget[];
}

export interface MiningLoopTopic {
  status: 'ok' | 'idle';
  plan: MiningLoopPlan | null;
}

export interface MiningCargoSellTopic {
  status: 'ok' | 'idle';
  commodities_in_cargo: CargoMineralItem[];
  best_multi_stations: MiningMultiSellHit[];
}

export type MiningTab = 'laser' | 'core' | 'missions';

export interface MiningPreset {
  id: string;
  tab: MiningTab;
  commodity: string;
  ringType: string;
  methodLabel: string;
  defaultResCombo?: boolean;
}

export const MINING_PRESETS: MiningPreset[] = [
  // Láser (Créditos)
  { id: 'plat-laser', tab: 'laser', commodity: 'Platinum', ringType: 'Metallic', methodLabel: 'Láser de Superficie', defaultResCombo: true },
  { id: 'painite-laser', tab: 'laser', commodity: 'Painite', ringType: 'Metallic', methodLabel: 'Láser de Superficie', defaultResCombo: false },
  { id: 'ltd-laser', tab: 'laser', commodity: 'Low Temperature Diamonds', ringType: 'Icy', methodLabel: 'Láser / Sub-superficie', defaultResCombo: false },
  // Núcleo Profundo
  { id: 'musgravite-core', tab: 'core', commodity: 'Musgravite', ringType: 'Rocky', methodLabel: 'Carga Sísmica (Núcleo)' },
  { id: 'monazite-core', tab: 'core', commodity: 'Monazite', ringType: 'Rocky', methodLabel: 'Carga Sísmica (Núcleo)' },
  { id: 'alexandrite-core', tab: 'core', commodity: 'Alexandrite', ringType: 'Metal-Rich', methodLabel: 'Carga Sísmica (Núcleo)' },
  { id: 'void-opal-core', tab: 'core', commodity: 'Void Opal', ringType: 'Icy', methodLabel: 'Carga Sísmica (Núcleo)' },
  { id: 'grandidierite-core', tab: 'core', commodity: 'Grandidierite', ringType: 'Metal-Rich', methodLabel: 'Carga Sísmica (Núcleo)' },
  // Misiones y Desbloqueos
  { id: 'bromellite-unlock', tab: 'missions', commodity: 'Bromellite', ringType: 'Icy', methodLabel: 'Ingeniero Bill Turner (50 t)' },
  { id: 'osmium-missions', tab: 'missions', commodity: 'Osmium', ringType: 'Metallic', methodLabel: 'Misiones de Facción y Colonización' },
];
