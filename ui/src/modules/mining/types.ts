export type Content = 'low' | 'medium' | 'high' | 'unknown';

export interface MaterialRow {
  key: string;
  name: string | null;
  /** Porcentaje 0-100. */
  proportion: number;
  est_value: number | null;
  interest: boolean;
}

export type Verdict =
  | { kind: 'worth'; est_value: number | null }
  | { kind: 'not_worth' }
  | { kind: 'no_data' };

export interface AsteroidView {
  content: Content;
  motherlode: string | null;
  materials: MaterialRow[];
  /** Porcentaje 0-100. */
  remaining: number | null;
  verdict: Verdict;
  est_total: number | null;
}

/** Topic `mining.prospector`: `history` con el más reciente primero (máx. 8). */
export interface ProspectorTopic {
  current: AsteroidView | null;
  history: AsteroidView[];
}

/** Topic `mining.session`. */
export interface MiningSessionTopic {
  started_at: string | null;
  refined: Record<string, number>;
  limpets_launched: Record<string, number>;
  limpets_left: number | null;
  cargo: number | null;
  capacity: number | null;
  asteroids_prospected: number;
  cracked: number;
  tons_per_hour: number | null;
  est_credits: number | null;
  credits_per_hour: number | null;
  /** Más reciente primero. */
  refinery_bins: { key: string; tons: number }[];
}

export type PadSize = 'Small' | 'Medium' | 'Large';

export interface SellOption {
  station_name: string;
  system_name: string;
  distance_ly: number;
  sell_price: number;
  demand: number | null;
  pad: PadSize | null;
  /** `null`: nave o plataforma desconocidas. */
  pad_ok: boolean | null;
  updated_at: string | null;
}

export type PriceSource = 'remote' | 'local_sell' | 'local_market';

export interface SellPlan {
  by_commodity: Record<string, SellOption[]>;
  best_station_total: [string, number] | null;
  fetched_at: string;
  source: PriceSource;
}

/** Topic `mining.sell`: el plan solo viene con `status === 'ok'`. */
export type SellTopic =
  | ({ status: 'ok' } & SellPlan)
  | { status: 'no_data' | 'disabled' | 'loading' };

/** Nombre legible de una materia: localizado si se conoce; si no, la clave capitalizada. */
export function materialName(key: string, names: Map<string, string>): string {
  const known = names.get(key);
  if (known) return known;
  return key ? key.charAt(0).toUpperCase() + key.slice(1) : key;
}

/** Nombres localizados vistos en los asteroides prospectados. */
export function namesFrom(p: ProspectorTopic | undefined): Map<string, string> {
  const map = new Map<string, string>();
  if (!p) return map;
  for (const a of [...(p.current ? [p.current] : []), ...p.history]) {
    for (const m of a.materials) if (m.name) map.set(m.key, m.name);
  }
  return map;
}
