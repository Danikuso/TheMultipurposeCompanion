export interface CarrierOverviewTopic {
  carrier_id: number | null;
  callsign: string | null;
  name: string | null;
  current_system: string | null;
  current_body: string | null;
  docking_access: string | null;
  fuel_depot: number;
  fuel_cargo: number;
  fuel_total: number;
  max_jumps_possible: number;
  cargo_free: number;
  cargo_total: number;
  cargo_used: number;
  is_aboard: boolean;
}

export type CarrierJumpStatus = 'idle' | 'counting_down' | 'locked' | 'cooldown';

export interface CarrierJumpTopic {
  status: CarrierJumpStatus;
  destination_system: string | null;
  destination_body: string | null;
  departure_time: string | null;
  seconds_remaining: number | null;
  cooldown_seconds_remaining: number;
  is_cancelled: boolean;
}

export type MaintenanceHealth = 'Safe' | 'Warning' | 'Critical';

export interface CarrierFinancesTopic {
  balance: number;
  reserve_balance: number;
  available_balance: number;
  weekly_upkeep: number;
  maintenance_health: MaintenanceHealth;
  weeks_covered: number;
  next_payment_time: string;
}
