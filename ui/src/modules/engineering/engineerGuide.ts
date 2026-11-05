import type { EngineerUnlockRow } from './types';

export type GuideStage = 'discovery' | 'invitation' | 'unlock';
export type GuideStepState = 'confirmed' | 'current' | 'pending' | 'unknown';
export interface GuideStep { stage: GuideStage; state: GuideStepState; key: string | null }
export interface GuideAvailability {
  available: number | null;
  required: number;
  missing: number | null;
  percent: number | null;
}

/** Only milestones confirmed by the journal, never the order of its enum. */
export function buildGuideSteps(status: string, rows: readonly EngineerUnlockRow[]): GuideStep[] {
  const guide = rows.find(row => row.field === 'guide');
  const invitation = rows.find(row => row.field === 'requirement');
  const unlock = rows.find(row => row.field === 'invite');
  const states: GuideStepState[] = status === 'Unlocked' ? ['confirmed', 'confirmed', 'confirmed']
    : status === 'Invited' ? ['confirmed', 'confirmed', 'current']
    : status === 'Known' ? ['confirmed', 'current', 'pending']
    : ['unknown', 'unknown', 'unknown'];
  return [
    { stage: 'discovery', state: states[0], key: guide?.field === 'guide' ? guide.discovery_key : null },
    { stage: 'invitation', state: states[1], key: invitation && 'key' in invitation ? invitation.key : null },
    { stage: 'unlock', state: states[2], key: unlock && 'key' in unlock ? unlock.key : null },
  ];
}

export function guideAvailability(available: number | null, required: number): GuideAvailability {
  const stock = available !== null && Number.isFinite(available) && available >= 0 ? available : null;
  const need = Number.isFinite(required) && required > 0 ? required : 0;
  return {
    available: stock, required: need,
    missing: stock === null || need === 0 ? null : Math.max(0, need - stock),
    percent: stock === null || need === 0 ? null : Math.min(100, stock / need * 100),
  };
}

export function guideRank(rank: number | null, progress: number | null): {
  rank: number | null; maximum: boolean; progress: number | null;
} {
  const knownRank = rank !== null && Number.isInteger(rank) && rank >= 1 && rank <= 5 ? rank : null;
  const maximum = knownRank === 5;
  return {
    rank: knownRank, maximum,
    progress: knownRank === null || maximum || progress === null || !Number.isFinite(progress)
      ? null : Math.max(0, Math.min(100, progress)),
  };
}
