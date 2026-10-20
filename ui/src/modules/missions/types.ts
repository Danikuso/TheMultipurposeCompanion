// Contratos de los topics `missions.*` (crates/tmc-modules/src/missions). Ninguno lleva
// campos dependientes del reloj: cuentas atrás y antigüedades se calculan en la UI.

export type MissionKind = 'Massacre' | 'Sightseeing' | 'Passenger' | 'Delivery' | 'Collect' | 'Courier' | 'Other';

interface Stamped {
  /** Último instante del diario (base de tiempo); época cero sin eventos. */
  as_of?: string | null;
}

export interface MissionProgress {
  delivered: number;
  collected: number;
  total: number | null;
  kills_needed: number | null;
  passengers: number | null;
}

export interface Mission {
  id: number;
  kind: MissionKind;
  name_localised: string;
  faction: string | null;
  destination_system: string | null;
  destination_station: string | null;
  stops: string[];
  accepted_at: string | null;
  expiry: string | null;
  reward: number | null;
  progress: MissionProgress;
  redirected: boolean;
  details_missing: boolean;
  target_faction: string | null;
  wing: boolean;
  commodity: string | null;
}

export interface MissionGroup {
  system: string | null;
  station: string | null;
  distance_ly: number | null;
  expiry: string | null;
  next: boolean;
  missions: Mission[];
}

export interface ActiveTopic extends Stamped {
  count: number;
  groups: MissionGroup[];
  next_system: string | null;
  copy_target: string | null;
  copy_seq: number;
}

export interface IssuerBar {
  faction: string;
  kills: number;
  needed: number;
  missions: number;
}

export interface MassacrePile {
  target_faction: string;
  issuers: IssuerBar[];
  kills_remaining: number;
  total_reward: number;
  details_missing: boolean;
}

export interface MassacreTopic extends Stamped {
  piles: MassacrePile[];
}

export type AlertKind =
  | 'ExpiringSoon'
  | 'Failed'
  | { Redirected: { system: string; station: string | null } }
  | { CargoMissing: { commodity: string; missing: number } };

export interface MissionAlertItem {
  mission_id: number;
  kind: AlertKind;
  at: string;
  bucket: number;
  name: string | null;
  mission_kind: MissionKind | null;
}

export interface AlertsTopic extends Stamped {
  alerts: MissionAlertItem[];
}

export type CabinClass = 'Economy' | 'Business' | 'First' | 'Luxury';

export interface CabinOccupancy {
  occupied: number | null;
  seats: number;
}

export interface Cabins {
  by_class: Partial<Record<CabinClass, CabinOccupancy>>;
  occupied_total: number;
  seats_total: number;
}

export interface PassengerGroup {
  destination: string | null;
  count: number;
  vip: boolean;
  wanted: boolean;
  mission_id: number;
}

export interface PassengersTopic extends Stamped {
  cabins: Cabins;
  groups: PassengerGroup[];
  wanted: boolean;
}

export interface TourismPlan {
  mission_id: number;
  order: string[];
  board_order_ly: number;
  optimal_ly: number;
  jumps: number | null;
  /** `null` si la ruta mide 0 al. */
  cr_per_ly: number | null;
  /** Estimación (≈). */
  cr_per_hour: number | null;
  estimates_verified: boolean;
  reference_cr_ly: number;
  reference_is_own: boolean;
  verdict: 'profitable' | 'not_worth';
  can_abandon_now: boolean;
}

export type TripStatus = 'ok' | 'loading' | 'no_coords' | 'stops_unavailable';

export interface Trip {
  mission_id: number;
  name: string;
  stops: string[];
  status: TripStatus;
  plan: TourismPlan | null;
}

export interface TourismTopic extends Stamped {
  trips: Trip[];
}

export interface FactionDelta {
  reputation: string;
  influence: string;
  reputation_score: number;
  influence_score: number;
}

export interface RewardsTopic extends Stamped {
  credits: number;
  completed: number;
  failed: number;
  abandoned: number;
  materials: Record<string, number>;
  factions: Record<string, FactionDelta>;
  since: string | null;
}

export interface CgView {
  cgid: number;
  title: string;
  system: string | null;
  station: string | null;
  contribution: number;
  percentile: number | null;
  tier: string | null;
  expiry: string | null;
  bonus: number | null;
  complete: boolean;
}

export interface CommunityGoalsTopic extends Stamped {
  goals: CgView[];
}
