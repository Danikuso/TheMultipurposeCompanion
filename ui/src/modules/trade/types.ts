// Contratos de los topics `trade.*` (crates/tmc-modules/src/trade). Ninguno lleva campos
// dependientes del reloj: antigüedades y créditos/hora se calculan en la UI.

export type PadSize = 'Small' | 'Medium' | 'Large';
export type PriceOrigin = 'Local' | 'Eddn' | 'Ardent';

/** Opción de venta (misma forma que `mining.sell`). */
export interface SellOption {
  station_name: string;
  system_name: string;
  distance_ly: number;
  sell_price: number;
  demand: number | null;
  pad: PadSize | null;
  /** `null` si la nave o la plataforma se desconocen. */
  pad_ok: boolean | null;
  updated_at: string | null;
  origin: PriceOrigin | null;
}

interface Stamped {
  /** Último instante del diario (base de tiempo); época cero sin eventos. */
  as_of?: string | null;
}

export interface MarketRow {
  commodity: string;
  name_localised: string | null;
  buy_price: number;
  stock: number;
  best_sell: SellOption | null;
  profit_per_ton: number | null;
  /** Distancia mínima recomendada de venta (al) en mercancías raras. */
  rare: number | null;
}

/** `trade.market`; sin mercado, `station` y `system` son `null`. */
export interface MarketTopic extends Stamped {
  station: string | null;
  system: string | null;
  rows: MarketRow[];
  eddn_connected?: boolean;
}

export type CargoKind = 'Trade' | 'Mission' | 'Stolen';
export type CargoAdvice = 'SellHere' | 'Carry';

export interface CargoLine {
  commodity: string;
  name_localised: string | null;
  tons: number;
  paid_avg: number | null;
  paid_partial: boolean;
  price_here: number | null;
  best_nearby: SellOption | null;
  est_profit: number | null;
  advice: CargoAdvice | null;
  kind: CargoKind;
}

/** `trade.cargo`. */
export interface CargoTopic extends Stamped {
  lines: CargoLine[];
  used: number;
  capacity: number | null;
}

export interface TradeCommodity {
  commodity: string;
  name_localised: string | null;
  amount: number;
  buy_price: number;
  sell_price: number;
  profit_per_ton: number;
}

export interface TradeStop {
  from_system: string;
  from_station: string;
  to_system: string;
  to_station: string;
  commodities: TradeCommodity[];
  stop_profit: number;
  distance_ly: number;
  data_updated_at: string | null;
}

export type RouteStatus = 'idle' | 'blocked' | 'loading' | 'ok' | 'no_data' | 'disabled';

/** `trade.route`. */
export interface RouteTopic extends Stamped {
  plan: { stops: TradeStop[]; total_profit: number } | null;
  /** Índice (desde 0) de la parada actual. */
  current: number;
  completed: boolean;
  copy_target: string | null;
  copy_seq: number;
  status: RouteStatus;
  blocker: 'NotDocked' | 'NoShip' | 'NoCredits' | null;
}

/** `trade.session`. */
export interface TradeSessionTopic extends Stamped {
  started_at: string | null;
  profit: number;
  revenue: number;
  cost_of_sold: number;
  tons_bought: number;
  tons_sold: number;
  operations: number;
  best_trade: [string, number] | null;
  unknown_cost_tons: number;
  /** 0-8 (Penniless … Elite). */
  rank: number | null;
  /** Porcentaje 0-100 hacia el siguiente rango. */
  progress: number | null;
}

export interface BuyOption {
  station_name: string;
  system_name: string;
  distance_ly: number;
  buy_price: number;
  stock: number;
  pad: PadSize | null;
  pad_ok: boolean | null;
  updated_at: string | null;
  origin: PriceOrigin;
}

/** `trade.search`. */
export interface SearchTopic extends Stamped {
  commodity: string | null;
  status: 'idle' | 'loading' | 'ok' | 'no_data';
  rows: BuyOption[];
}

/** `trade.commodities`: pares [clave, nombre localizado]. */
export interface CommoditiesTopic extends Stamped {
  items: [string, string | null][];
}

export function commodityName(key: string, localised: string | null | undefined): string {
  if (localised) return localised;
  return key ? key.charAt(0).toUpperCase() + key.slice(1) : key;
}
