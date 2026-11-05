import { describe, expect, test } from 'vitest';
import { buildGuideSteps, guideAvailability, guideCargoStock, guideRank } from '../src/modules/engineering/engineerGuide';
import type { EngineerUnlockRow } from '../src/modules/engineering/types';

const rows: EngineerUnlockRow[] = [
  { engineer_id: 300100, field: 'guide', discovery_key: 'discover', reputation_key: 'reputation', verified: true },
  { engineer_id: 300100, field: 'requirement', key: 'invite' },
  { engineer_id: 300100, field: 'invite', key: 'unlock' },
];
describe('journal-confirmed engineer milestones', () => {
  test.each([
    ['Known', ['confirmed', 'current', 'pending']],
    ['Invited', ['confirmed', 'confirmed', 'current']],
    ['Unlocked', ['confirmed', 'confirmed', 'confirmed']],
    ['Acquainted', ['unknown', 'unknown', 'unknown']],
    ['Barred', ['unknown', 'unknown', 'unknown']],
    ['Unknown', ['unknown', 'unknown', 'unknown']],
    ['FutureStatus', ['unknown', 'unknown', 'unknown']],
  ])('%s does not infer more progress than the journal confirms', (status, states) => {
    const steps = buildGuideSteps(status as string, rows);
    expect(steps.map(s => s.state)).toEqual(states);
    expect(steps.map(s => s.key)).toEqual(['discover', 'invite', 'unlock']);
    expect(steps.map(s => s.stage)).toEqual(['discovery', 'invitation', 'unlock']);
  });
  test('missing guide fields remain missing instead of exposing a made-up requirement', () => {
    expect(buildGuideSteps('Unknown', []).map(s => s.key)).toEqual([null, null, null]);
  });
});
describe('stock is not a delivered contribution', () => {
  test('missing inventory stays unknown', () => {
    expect(guideAvailability(null, 10)).toEqual({ available: null, required: 10, missing: null, percent: null });
  });
  test('known stock yields exact deficit and a capped meter', () => {
    expect(guideAvailability(3, 10)).toEqual({ available: 3, required: 10, missing: 7, percent: 30 });
    expect(guideAvailability(12, 10)).toEqual({ available: 12, required: 10, missing: 0, percent: 100 });
    expect(guideAvailability(0, 10).missing).toBe(10);
  });
  test('invalid quantities never produce NaN or an infinite percentage', () => {
    expect(guideAvailability(Number.NaN, 10).available).toBeNull();
    expect(guideAvailability(-1, 10).available).toBeNull();
    expect(guideAvailability(3, 0).percent).toBeNull();
  });
});
test('rank and progress are separate, bounded and honest when absent', () => {
  expect(guideRank(null, null)).toEqual({ rank: null, progress: null, maximum: false });
  expect(guideRank(3, 42)).toEqual({ rank: 3, progress: 42, maximum: false });
  expect(guideRank(5, null)).toEqual({ rank: 5, progress: null, maximum: true });
  expect(guideRank(5, 100).progress).toBeNull();
  expect(guideRank(3, 200).progress).toBe(100);
  expect(guideRank(3, -1).progress).toBe(0);
  expect(guideRank(3, Number.NaN).progress).toBeNull();
  expect(guideRank(0, 50).rank).toBeNull();
});
test('cargo from live journal trade inventory takes precedence over an unpopulated companion list', () => {
  expect(guideCargoStock('metaalloys', { cargo_count: 3, cargo: [] }, {
    used: 3, lines: [{ commodity: 'metaalloys', tons: 3, kind: 'Trade' }],
  })).toBe(3);
});
test('an unconfirmed empty inventory is unknown, but an authoritative zero count is zero', () => {
  expect(guideCargoStock('metaalloys', { cargo_count: null, cargo: [] }, { used: 0, lines: [] })).toBeNull();
  expect(guideCargoStock('metaalloys', { cargo_count: 0, cargo: [] }, undefined)).toBe(0);
  expect(guideCargoStock('metaalloys', { cargo_count: 5, cargo: [] }, { used: 3, lines: [{ commodity: 'metaalloys', tons: 3, kind: 'Trade' }] })).toBeNull();
});
test('a complete companion inventory is a fallback and split cargo lines count physical stock exactly once', () => {
  expect(guideCargoStock('metaalloys', { cargo_count: 3, cargo: [{ Name: 'metaalloys', Count: 3 }] }, undefined)).toBe(3);
  expect(guideCargoStock('gold', { cargo_count: 9, cargo: [] }, { used: 9, lines: [
    { commodity: 'gold', tons: 3, kind: 'Trade' }, { commodity: 'gold', tons: 2, kind: 'Stolen' }, { commodity: 'gold', tons: 4, kind: 'Mission' },
  ] })).toBe(9);
  expect(guideCargoStock('gold', { cargo_count: 9, cargo: [{ Name: 'gold', Count: 5, Stolen: 2 }, { Name: 'gold', Count: 4, MissionID: 1 }] }, undefined)).toBe(9);
});
