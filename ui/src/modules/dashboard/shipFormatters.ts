import { t } from '../../lib/i18n';

export interface FormattedModule {
  slot: string;
  slotLabel: string;
  category: 'hardpoint' | 'core' | 'optional' | 'utility';
  item: string;
  itemLabel: string;
  sizeClass: string;
  healthPct: number;
  on: boolean;
  priority: number;
  ammo: string | null;
  engineering?: {
    blueprint: string;
    level: number;
    engineer?: string;
    experimental?: string;
    modifiers?: { label: string; value: number | null; original: number | null; lessIsGood?: boolean }[];
  } | null;
}

export function categorizeSlot(slot: string): 'hardpoint' | 'core' | 'optional' | 'utility' {
  const s = slot.toLowerCase();
  if (s.startsWith('tinyhardpoint')) return 'utility';
  if (s.includes('hardpoint')) return 'hardpoint';
  if (
    s.includes('powerplant') ||
    s.includes('mainengine') ||
    s.includes('frameshiftdrive') ||
    s.includes('lifesupport') ||
    s.includes('powerdistributor') ||
    s.includes('radar') ||
    s.includes('fueltank') ||
    s.includes('armour')
  ) {
    return 'core';
  }
  return 'optional';
}

export function formatSlotName(slot: string): string {
  const s = slot.toLowerCase();
  const number = slot.replace(/\D/g, '') || '1';
  for (const size of ['huge', 'large', 'medium', 'small', 'tiny']) {
    if (s.startsWith(size + 'hardpoint')) return t('ship-slot-hardpoint-' + size, { number });
  }
  const core: Record<string, string> = {
    armour: 'modtype-armour', powerplant: 'modtype-powerplant', mainengines: 'modtype-engine',
    frameshiftdrive: 'modtype-fsd', lifesupport: 'modtype-lifesupport', powerdistributor: 'modtype-powerdistributor',
    radar: 'modtype-sensor', fueltank: 'ship-module-fuel-tank', planetaryapproachsuite: 'ship-module-planetary-approach',
    cargohatch: 'combat-subsystem-cargohatch',
  };
  if (core[s]) return t(core[s]);
  if (s.startsWith('military')) return t('ship-slot-military', { number });
  if (s.startsWith('slot')) {
    const parts = slot.split('_');
    const slotNumber = parts[0]?.replace(/\D/g, '') || '';
    const size = parts[1]?.replace(/\D/g, '') || '';
    return size ? t('ship-slot-optional-sized', { number: slotNumber, size }) : t('ship-slot-optional', { number: slotNumber });
  }
  return slot;
}

const CLASS_LETTERS: Record<number, string> = { 1: 'E', 2: 'D', 3: 'C', 4: 'B', 5: 'A' };

const MODULE_LABELS: [string, string, string?][] = [
  ['hyperdrive', 'modtype-fsd'], ['powerplant', 'modtype-powerplant'], ['engine', 'modtype-engine'],
  ['lifesupport', 'modtype-lifesupport'], ['powerdistributor', 'modtype-powerdistributor'],
  ['sensors', 'modtype-sensor'], ['radar', 'modtype-sensor'], ['fueltank', 'ship-module-fuel-tank'],
  ['shieldgenerator', 'modtype-shieldgenerator'], ['fuelscoop', 'modtype-fuelscoop'],
  ['guardianfsdbooster', 'ship-module-guardian-fsd'], ['detailedsurfacescanner', 'modtype-surfacescanner', '1I'],
  ['shieldbooster', 'modtype-shieldbooster'], ['chafflauncher', 'modtype-chafflauncher', '0I'],
  ['heatsinklauncher', 'modtype-heatsinklauncher', '0I'], ['pointdefence', 'modtype-pointdefence', '0I'],
  ['electroniccountermeasure', 'modtype-electroniccountermeasure', '0I'], ['shieldcellbank', 'modtype-shieldcellbank'],
  ['hullreinforcement', 'modtype-hullreinforcementpackage'], ['modulereinforcement', 'modtype-modulereinforcementpackage'],
  ['repairer', 'modtype-autofieldmaintenanceunit'], ['refinery', 'modtype-refinery'],
  ['plasmaaccelerator', 'modtype-plasmaaccelerator'], ['fragmentcannon', 'modtype-fragmentcannon'], ['cannon', 'modtype-cannon'],
  ['railgun', 'modtype-railgun'], ['missile', 'modtype-missilerack'], ['torpedo', 'modtype-torpedopylon'],
  ['mininglaser', 'modtype-mininglaser'], ['subsurfacemining', 'ship-module-subsurface'],
  ['seismiccharge', 'ship-module-seismic'], ['abrasionblaster', 'modtype-abrasionblaster'],
  ['dronecontrol_collection', 'modtype-collectorlimpetcontroller'], ['dronecontrol_prospector', 'modtype-prospectorlimpetcontroller'],
  ['dronecontrol_repair', 'ship-module-repair-limpet'], ['buggybay', 'ship-module-srv-hangar'], ['fighterbay', 'ship-module-fighter-hangar'],
  ['dockingcomputer', 'ship-module-docking-computer', '1E'], ['supercruiseassist', 'ship-module-supercruise-assist', '1E'],
  ['passengercabin', 'ship-module-passenger-cabin'], ['planetapproachsuite', 'ship-module-planetary-approach', '1I'],
  ['armour_grade1', 'ship-module-armour-lightweight', '1I'], ['armour_grade2', 'ship-module-armour-reinforced', '1I'],
  ['armour_grade3', 'ship-module-armour-military', '1I'],
];


export function formatItemName(rawItem: string): { label: string; sizeClass: string } {
  if (!rawItem) return { label: '—', sizeClass: '' };
  const lower = rawItem.toLowerCase();
  const size = lower.match(/size(\d+)/)?.[1] ?? '';
  const classNum = Number(lower.match(/class(\d+)/)?.[1]) || null;
  const classLetter = classNum ? (CLASS_LETTERS[classNum] ?? String(classNum)) : '';
  const sizeClass = size && classLetter ? size + classLetter : size || classLetter;
  if (lower.includes('cargorack')) {
    const tons = size ? Math.pow(2, Number(size)) : null;
    return { label: tons === null ? t('modtype-cargorack') : t('ship-module-cargo-capacity', { tons }), sizeClass };
  }
  for (const [token, module] of [['multicannon', 'multi_cannon'], ['beamlaser', 'beamlaser'], ['pulselaser', 'pulselaser'], ['burstlaser', 'burstlaser']]) {
    if (lower.includes(token)) {
      const mount = lower.includes('turret') ? 'turret' : lower.includes('gimbal') ? 'gimbal' : 'fixed';
      return { label: t('ship-module-mounted', { module: t('modtype-' + module), mount: t('outfitting-mount-' + mount) }), sizeClass };
    }
  }
  for (const [token, key, fixedClass] of MODULE_LABELS) if (lower.includes(token)) return { label: t(key), sizeClass: fixedClass ?? sizeClass };
  const clean = rawItem.replace(/^(int_|hpt_)/, '').replace(/_/g, ' ');
  return { label: clean.charAt(0).toUpperCase() + clean.slice(1), sizeClass };
}
