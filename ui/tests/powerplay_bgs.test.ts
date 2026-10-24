import type { Component } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import PowerplayPanel from '../src/modules/powerplay/PowerplayPanel.svelte';
import BgsPanel from '../src/modules/bgs/BgsPanel.svelte';

let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;

function mountWith(comp: Component<any>, topics: Record<string, unknown>) {
  for (const [k, v] of Object.entries(topics)) live.topics[k] = v;
  mounted = mount(comp, { target });
  flushSync();
}

const q = (id: string) => target.querySelector(`[data-testid="${id}"]`);
const qa = (id: string) => [...target.querySelectorAll(`[data-testid="${id}"]`)];

beforeEach(() => {
  disconnect();
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  if (mounted) unmount(mounted);
  mounted = undefined;
  target.remove();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

test('PowerplayPanel shows empty state when not pledged', () => {
  mountWith(PowerplayPanel, { 'powerplay.status': null });
  expect(q('powerplay-no-data')).toBeTruthy();
});

test('PowerplayPanel renders power name, rank 24, merits, and session rate', () => {
  mountWith(PowerplayPanel, {
    'powerplay.status': {
      power: 'Aisling Duval',
      rank: 24,
      merits: 3120,
      merits_for_current_rank: 3000,
      merits_for_next_rank: 3300,
      merits_needed_for_next: 180,
      rank_progress_percent: 40.0,
      time_pledged_seconds: 14 * 86400,
      time_pledged_weeks: 2,
      seconds_to_cycle_end: 259200, // 3d
      next_cycle_time: '2026-10-29T07:00:00Z',
    },
    'powerplay.session': {
      session_merits: 450,
      session_seconds: 3600,
      merits_per_hour: 450,
    },
  });

  expect(q('powerplay-power')?.textContent).toBe('Aisling Duval');
  expect(q('powerplay-weeks')?.textContent).toContain('2');
  expect(q('powerplay-rank')?.textContent).toContain('24');
  expect(q('powerplay-merits')?.textContent).toContain('3.120');
  expect(q('powerplay-needed')?.textContent).toContain('180');
  expect(q('powerplay-session-merits')?.textContent).toContain('+450');
  expect(q('powerplay-merits-rate')?.textContent).toContain('450');
  expect(q('powerplay-cycle-timer')?.textContent).toContain('3d');
});

test('BgsPanel shows empty state when no system data', () => {
  mountWith(BgsPanel, { 'bgs.system': null });
  expect(q('bgs-no-data')).toBeTruthy();
});

test('BgsPanel renders star system, tick card and factions ordered by influence', () => {
  mountWith(BgsPanel, {
    'bgs.system': {
      star_system: 'Barnard\'s Star',
      system_address: 1234567,
      controlling_faction: 'Barnard\'s Star Alliance',
      controlling_faction_state: 'Election',
      factions_count: 2,
      factions: [
        {
          name: 'Barnard\'s Star Alliance',
          influence_percent: 55.0,
          state: 'Election',
          allegiance: 'Federation',
          government: 'Democracy',
          happiness: 'Happy',
          is_controlling: true,
        },
        {
          name: 'Barnard Pirates',
          influence_percent: 20.0,
          state: 'War',
          allegiance: 'Independent',
          government: 'Anarchy',
          happiness: 'Unhappy',
          is_controlling: false,
        },
      ],
    },
    'bgs.tick': {
      latest_tick: '2026-10-23T04:00:00Z',
      updated_at: '2026-10-23T10:00:00Z',
      seconds_since_tick: 6 * 3600,
      hours_since_tick: 6.0,
      is_stale: false,
    },
  });

  expect(q('bgs-system-name')?.textContent).toBe('Barnard\'s Star');
  expect(q('bgs-controlling')?.textContent).toBe('Barnard\'s Star Alliance');
  expect(q('bgs-tick-hours')?.textContent).toContain('6');
  expect(q('bgs-tick-badge')?.textContent).toContain('Reciente');

  const rows = qa('faction-row');
  expect(rows.length).toBe(2);
  expect(rows[0].textContent).toContain('Barnard\'s Star Alliance');
  expect(rows[0].textContent).toContain('55');
  expect(rows[0].querySelector('[data-testid="controlling-badge"]')).toBeTruthy();

  expect(rows[1].textContent).toContain('Barnard Pirates');
  expect(rows[1].textContent).toContain('20');
  expect(rows[1].textContent).toContain('War');
});
