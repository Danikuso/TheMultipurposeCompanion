export interface BodyRow {
  body_id: number;
  short_name: string;
  kind_key: string;
  star_class?: string;
  distance_ls?: number;
  scanned_value: number;
  mapped_value?: number;
  bio?: number;
  geo?: number;
  marks?: number;
  probes?: { n: number; estimated?: boolean };
  advice?: ('map' | 'land' | 'not_worth')[];
}

export interface SystemTopic {
  system: string;
  system_address: number;
  discovered?: boolean;
  fss: { progress?: number; body_count?: number; scanned: number; complete: boolean };
  value_now: number;
  value_if_mapped: number;
  unidentified?: number;
  rows: BodyRow[];
}

export interface UnsoldTopic { total: number; systems: number; at_risk: boolean }
