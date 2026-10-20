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
  /** Último instante del diario (base de tiempo); ausente o época cero sin eventos. */
  as_of?: string | null;
  /** Clave de materia -> nombre localizado aprendido del diario. */
  names?: Record<string, string>;
  started_at: string | null;
  refined: Record<string, number>;
  limpets_launched: Record<string, number>;
  limpets_left: number | null;
  cargo: number | null;
  capacity: number | null;
  asteroids_prospected: number;
  cracked: number;
  est_credits: number | null;
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
  /** Procedencia del precio; `null` si no se conoce. */
  origin?: 'Local' | 'Eddn' | 'Ardent' | null;
  updated_at: string | null;
}

export type PriceSource = 'remote' | 'local_sell' | 'local_market';

export interface SellPlan {
  by_commodity: Record<string, SellOption[]>;
  best_station_total: [string, number] | null;
  fetched_at: string | null;
  source: PriceSource;
}

/** Topic `mining.sell`: el plan solo viene con `status === 'ok'`. */
export interface SellBase {
  as_of?: string | null;
  names?: Record<string, string>;
}
export type SellTopic =
  | ({ status: 'ok' } & SellBase & SellPlan)
  | ({ status: 'no_data' | 'disabled' | 'loading' } & SellBase);

/** Nombre legible de una materia: localizado si se conoce; si no, la clave capitalizada. */
export function materialName(key: string, names: Map<string, string>): string {
  const known = names.get(key);
  if (known) return known;
  return key ? key.charAt(0).toUpperCase() + key.slice(1) : key;
}

/** Nombres localizados: los del backend (`names`) y, tras ellos, los de los asteroides. */
export function namesFrom(
  p: ProspectorTopic | undefined,
  known: Record<string, string> | undefined = undefined,
): Map<string, string> {
  const map = new Map<string, string>();
  for (const [k, v] of Object.entries(known ?? {})) if (v) map.set(k, v);
  if (!p) return map;
  for (const a of [...(p.current ? [p.current] : []), ...p.history]) {
    for (const m of a.materials) if (m.name && !map.has(m.key)) map.set(m.key, m.name);
  }
  return map;
}
