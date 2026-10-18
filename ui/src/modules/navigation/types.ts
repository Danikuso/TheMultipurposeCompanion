export interface Politics {
  allegiance?: string | null;
  government?: string | null;
  controlling_faction?: string | null;
  faction_state?: string | null;
  economy?: string | null;
  second_economy?: string | null;
  security?: string | null;
  population?: number | null;
  powers?: string[];
  powerplay_state?: string | null;
}

export interface RemoteData extends Politics {
  discovered?: boolean | null;
  body_count?: number | null;
}

/** Ficha de sistema del topic `navigation.route` (ver `strip.rs`). */
export interface Card {
  name: string;
  jump_ly: number | null;
  star_class: string | null;
  scoopable: boolean | null;
  special: 'neutron' | 'white_dwarf' | null;
  fuel_after_pct: number | null;
  reachable: boolean | null;
  visits: number | null;
  remote: {
    status: 'pending' | 'ok' | 'no-data' | 'disabled' | 'failed';
    source: string | null;
    fetched_at: string | null;
    data: RemoteData | null;
  };
  local_politics: Politics | null;
}

export interface RouteTopic {
  source: 'plan' | 'nav-route' | 'current';
  current: Card;
  next: Card | null;
  following: Card[];
  remaining_jumps: number | null;
  total_ly: number | null;
  off_route: boolean;
}

export interface JumpTopic {
  range_ly: number | null;
  range_full_ly: number | null;
  source: 'formula' | 'scaled' | 'unknown';
  fuel_main: number | null;
  fuel_capacity: number | null;
  max_fuel_per_jump: number | null;
  supercharged: number | null;
}
