export interface Candidate {
  species: string;
  species_name: string | null;
  genus: string;
  genus_name: string | null;
  value: number;
  value_x5: number;
  min_m: number;
  likelihood: 'high' | 'medium' | 'low' | 'unknown';
  sources: ('rules' | 'history' | 'canonn')[];
  status: 'none' | 'in-progress' | 'done';
  progress: number | null;
}

export interface BodyTopic {
  body: string | null;
  body_id: number;
  system_address: number;
  kind: string | null;
  gravity_g: number | null;
  temperature_k: number | null;
  atmosphere: string | null;
  bio_signals: number;
  first_logged: boolean | null;
  dss: boolean;
  candidates: Candidate[];
  confirmed_genera: { genus: string; name: string | null }[];
  completed: number;
}

export interface SamplingTopic {
  species: string;
  species_name: string | null;
  genus: string;
  genus_name: string | null;
  body_id: number;
  points: number;
  complete: boolean;
  distance_m: number | null;
  min_m: number;
  remaining_m: number | null;
  direction: { rel: string; compass: string } | null;
  unsold_after: number;
  discarded_species: string | null;
  discarded_name: string | null;
}

export interface UnsoldTopic { total: number; species: number }
