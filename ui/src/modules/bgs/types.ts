export interface BgsFaction {
  name: string;
  influence_percent: number;
  state: string | null;
  allegiance: string | null;
  government: string | null;
  happiness: string | null;
  is_controlling: boolean;
}

export interface BgsSystemTopic {
  star_system: string | null;
  system_address: number | null;
  controlling_faction: string | null;
  controlling_faction_state: string | null;
  factions_count: number;
  factions: BgsFaction[];
}

export interface BgsTickTopic {
  latest_tick: string | null;
  updated_at: string | null;
  seconds_since_tick: number | null;
  hours_since_tick: number | null;
  is_stale: boolean;
}
