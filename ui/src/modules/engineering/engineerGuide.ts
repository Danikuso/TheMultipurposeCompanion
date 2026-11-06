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
export interface GuideShipCargo {
  cargo_count?: number | null;
  cargo?: { Name: string; Count: number; Stolen?: number; MissionID?: number | null }[];
}
export interface GuideTradeCargo {
  used: number;
  lines: { commodity: string; tons: number; kind: string }[];
}

/** Journal Cargo updates trade inventory before Cargo.json populates the companion list. */
export function guideCargoStock(item: string, ship: GuideShipCargo | undefined, trade: GuideTradeCargo | undefined): number | null {
  const valid = (n: unknown): n is number => typeof n === 'number' && Number.isSafeInteger(n) && n >= 0;
  const count = valid(ship?.cargo_count) ? ship.cargo_count : null;
  if (count === 0) return 0;
  const key = item.toLowerCase();
  if (trade && valid(trade.used) && trade.used > 0 && Array.isArray(trade.lines)
    && trade.lines.every(line => valid(line.tons))
    && trade.lines.reduce((sum, line) => sum + line.tons, 0) === trade.used
    && (count === null || count === trade.used)) {
    return trade.lines.reduce((sum, line) => line.commodity.toLowerCase() === key ? sum + line.tons : sum, 0);
  }
  if (count !== null && ship?.cargo && ship.cargo.every(line => valid(line.Count))
    && ship.cargo.reduce((sum, line) => sum + line.Count, 0) === count) {
    return ship.cargo.reduce((sum, line) => line.Name.toLowerCase() === key ? sum + line.Count : sum, 0);
  }
  return null;
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
