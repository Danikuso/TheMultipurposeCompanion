export interface PowerplayStatusTopic {
  power: string | null;
  rank: number;
  merits: number;
  merits_for_current_rank: number;
  merits_for_next_rank: number;
  merits_needed_for_next: number;
  rank_progress_percent: number;
  time_pledged_seconds: number;
  time_pledged_weeks: number;
  seconds_to_cycle_end: number;
  next_cycle_time: string;
}

export interface PowerplaySessionTopic {
  session_merits: number;
  session_seconds: number;
  merits_per_hour: number;
}
