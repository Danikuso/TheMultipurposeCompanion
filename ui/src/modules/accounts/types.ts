export interface EddnStatus {
  enabled: boolean;
  connected: boolean;
  uploader_id: string;
  messages_sent_session: number;
  last_upload_time: string | null;
  last_error: string | null;
}

export interface EdsmStatus {
  enabled: boolean;
  has_key: boolean;
  commander_name: string | null;
  last_sync_time: string | null;
  last_error: string | null;
  syncing: boolean;
}

export interface InaraStatus {
  enabled: boolean;
  has_key: boolean;
  commander_name: string | null;
  last_sync_time: string | null;
  last_error: string | null;
  pending_events: number;
  next_slot_secs: number;
  syncing: boolean;
}

export interface RemoteShip {
  id: number;
  name: string | null;
  ship_type: string;
  station: string;
  system: string;
  value: number;
  distance_ly: number | null;
  transfer_time_secs: number | null;
}

export interface CapiStatus {
  configured: boolean;
  connected: boolean;
  commander_name: string | null;
  token_expires_at: string | null;
  remote_ships_count: number;
  last_sync_time: string | null;
  last_error: string | null;
  remote_ships?: RemoteShip[];
}

export interface AccountsStatusTopic {
  eddn: EddnStatus;
  edsm: EdsmStatus;
  inara: InaraStatus;
  capi: CapiStatus;
}
