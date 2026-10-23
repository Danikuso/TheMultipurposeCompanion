export interface ConstructionResource {
  name: string;
  name_localised: string | null;
  required: number;
  provided: number;
  remaining: number;
  progress_percent: number;
  trips_remaining: number;
}

export interface ColonisationDepotTopic {
  active_project: boolean;
  market_id: number | null;
  station_name: string | null;
  construction_type: string | null;
  progress_percent: number;
  total_required: number;
  total_provided: number;
  total_remaining: number;
  cargo_capacity: number;
  total_trips_remaining: number;
  is_docked: boolean;
  resources: ConstructionResource[];
}
