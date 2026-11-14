export type SyncServiceName = 'inara' | 'edsm' | 'eddn';

export type SyncState =
  | 'disabled'
  | 'paused'
  | 'waiting'
  | 'connected'
  | 'error'
  | 'unavailable';

export type SyncReason =
  | 'no_key'
  | 'switch_off'
  | 'online_off'
  | 'legacy'
  | 'beta'
  | 'replay'
  | 'game_not_running'
  | 'other_instance'
  | 'platform_unsupported'
  | 'key_unassigned'
  | 'network'
  | 'timeout'
  | 'rate_limited'
  | 'blocked'
  | 'auth'
  | 'software_blocked'
  | 'schema_outdated'
  | 'server_error'
  | 'discard_list_missing';

export type ErrorClass =
  | 'network'
  | 'timeout'
  | 'rate_limited'
  | 'blocked'
  | 'auth'
  | 'software_blocked'
  | 'schema_outdated'
  | 'invalid_event'
  | 'too_large'
  | 'server_error'
  | 'discard_list_missing';

export interface RecentRejection {
  kind: string;
  class: ErrorClass;
  code: string;
  at: string;
  hint?: string;
}

export interface SyncServiceStatus {
  state: SyncState;
  reason: SyncReason | null;
  pending: number;
  recovered_session: number;
  sent_total: number;
  sent_session: number;
  rejected_total: number;
  rejected_session: number;
  expired_total: number;
  omitted_total: number;
  last_confirmed_at: string | null;
  next_attempt_at: string | null;
  recent_rejections: RecentRejection[];
}

export type KeyedServiceStatus = SyncServiceStatus & {
  has_key: boolean;
  key_hint: string | null;
};

export type EddnServiceStatus = SyncServiceStatus & {
  enabled: boolean;
  paused_schemas: string[];
};

export interface CmdrRef {
  cmdr_ref: string;
  name: string;
}

export type KeyAssignment = 'confirmed' | 'auto_pending_confirmation';

export interface CommanderRow {
  cmdr_ref: string;
  name: string;
  inara_key: boolean;
  edsm_key: boolean;
  key_assignment: KeyAssignment | null;
  inara_pending: number;
  edsm_pending: number;
  eddn_pending: number;
}

export interface PendingByService {
  inara: number;
  edsm: number;
  eddn: number;
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
  current_cmdr: CmdrRef | null;
  inara: KeyedServiceStatus;
  edsm: KeyedServiceStatus;
  eddn: EddnServiceStatus;
  commanders: CommanderRow[];
  other_cmdrs_pending: PendingByService;
  unassigned_keys: SyncServiceName[];
  capi: CapiStatus;
}
