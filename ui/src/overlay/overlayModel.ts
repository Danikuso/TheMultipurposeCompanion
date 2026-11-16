import type { OverlayBlocks, OverlaySettings } from '../lib/prefs.svelte';
import { autoLayout, contextPanel } from '../shell/layouts';
import { DEFAULT_PRIORITIES } from '../shell/priorities';

export type OverlayRole =
  | 'exobiology'
  | 'exploration'
  | 'combat'
  | 'mining'
  | 'trade'
  | 'colonisation'
  | 'navigation';

export interface OverlayRoleContext {
  modules: { id: string; relevance: string }[];
  hasSamplingOrBio: boolean;
  hasCombatActivity: boolean;
  hasProspector: boolean;
  hasColonisation: boolean;
  hasTrade: boolean;
  hasExploration: boolean;
}

const TOGGLEABLE_ROLES: readonly (keyof OverlayBlocks)[] = [
  'exobiology',
  'combat',
  'mining',
  'colonisation',
  'trade',
  'exploration',
  'navigation',
];

export function isBlockEnabled(blocks: OverlayBlocks, role: string): boolean {
  if (role in blocks) {
    return blocks[role as keyof OverlayBlocks];
  }
  return blocks.navigation;
}

/** Resuelve qué bloque táctico mostrar según rol fijado, Auto-Companion y bloques activos. */
export function resolveOverlayRole(
  settings: OverlaySettings,
  ctx: OverlayRoleContext,
  priorities: string[] = DEFAULT_PRIORITIES,
): OverlayRole | null {
  const { blocks, pinned_role } = settings;

  const mapLayout = (id: string | null): OverlayRole | null => {
    if (!id) return null;
    if (id === 'exploration') {
      if ((contextPanel(ctx.modules, null) === 'exobiology' || ctx.hasSamplingOrBio) && blocks.exobiology) {
        return 'exobiology';
      }
      return 'exploration';
    }
    if (id === 'combat' || id === 'mining' || id === 'trade' || id === 'colonisation') {
      return id;
    }
    return 'navigation';
  };

  if (pinned_role) {
    const normalized = pinned_role.trim() as OverlayRole;
    if ((TOGGLEABLE_ROLES as readonly string[]).includes(normalized) && isBlockEnabled(blocks, normalized)) {
      return normalized;
    }
    if (isBlockEnabled(blocks, pinned_role)) {
      return mapLayout(pinned_role);
    }
  }

  const auto = mapLayout(autoLayout(ctx.modules, null, priorities, false));
  if (auto && isBlockEnabled(blocks, auto)) {
    return auto;
  }

  const fallbacks: [OverlayRole, boolean][] = [
    ['exobiology', ctx.hasSamplingOrBio],
    ['combat', ctx.hasCombatActivity],
    ['mining', ctx.hasProspector],
    ['colonisation', ctx.hasColonisation],
    ['trade', ctx.hasTrade],
    ['exploration', ctx.hasExploration],
    ['navigation', true],
  ];

  for (const [role, active] of fallbacks) {
    if (active && blocks[role]) {
      return role;
    }
  }

  return blocks.navigation ? 'navigation' : null;
}

export function distance3dLy(
  a: [number, number, number] | null | undefined,
  b: [number, number, number] | null | undefined,
): number | null {
  if (!a || !b || a.length !== 3 || b.length !== 3) return null;
  const dx = a[0] - b[0];
  const dy = a[1] - b[1];
  const dz = a[2] - b[2];
  const d = Math.hypot(dx, dy, dz);
  return Number.isFinite(d) ? d : null;
}
