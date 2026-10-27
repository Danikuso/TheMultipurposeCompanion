// ui/src/lib/icons/index.ts
// Ingesta de iconografía oficial de Elite Dangerous (EDAssets)

// Potencias (Powers - SVG vectoriales)
import aislingDuvalSvg from '../../assets/icons/powers/aisling-duval.svg?raw';
import archonDelaineSvg from '../../assets/icons/powers/archon-delaine.svg?raw';
import arissaLavignyDuvalSvg from '../../assets/icons/powers/arissa-lavigny-duval.svg?raw';
import dentonPatreusSvg from '../../assets/icons/powers/denton-patreus.svg?raw';
import edmundMahonSvg from '../../assets/icons/powers/edmund-mahon.svg?raw';
import feliciaWintersSvg from '../../assets/icons/powers/felicia-winters.svg?raw';
import liYongRuiSvg from '../../assets/icons/powers/li-yong-rui.svg?raw';
import pranavAntalSvg from '../../assets/icons/powers/pranav-antal.svg?raw';
import yuriGromSvg from '../../assets/icons/powers/yuri-grom.svg?raw';
import zacharyHudsonSvg from '../../assets/icons/powers/zachary-hudson.svg?raw';
import zeminaTorvalSvg from '../../assets/icons/powers/zemina-torval.svg?raw';

// Retratos oficiales (Portraits)
import aislingPortrait from '../../assets/icons/portraits/aisling-duval.jpg';
import feliciaPortrait from '../../assets/icons/portraits/felicia-winters.png';
import zacharyPortrait from '../../assets/icons/portraits/zachary-hudson.png';

// Superpotencias (Superpowers - SVG vectoriales)
import empireSvg from '../../assets/icons/superpowers/empire.svg?raw';
import federationSvg from '../../assets/icons/superpowers/federation.svg?raw';
import allianceSvg from '../../assets/icons/superpowers/alliance.svg?raw';

// Fabricantes de naves (Ship Manufacturers - SVG vectoriales)
import gutamayaSvg from '../../assets/icons/manufacturers/gutamaya.svg?raw';
import faulconDeLacySvg from '../../assets/icons/manufacturers/faulcon-delacy.svg?raw';
import coreDynamicsSvg from '../../assets/icons/manufacturers/core-dynamics.svg?raw';
import lakonSvg from '../../assets/icons/manufacturers/lakon.svg?raw';
import saudKrugerSvg from '../../assets/icons/manufacturers/saud-kruger.svg?raw';
import zorgonPetersonSvg from '../../assets/icons/manufacturers/zorgon-peterson.svg?raw';

// Estaciones (Stations - SVG vectoriales)
import coriolisSvg from '../../assets/icons/stations/coriolis.svg?raw';
import orbisSvg from '../../assets/icons/stations/orbis.svg?raw';
import ocellusSvg from '../../assets/icons/stations/ocellus.svg?raw';
import outpostSvg from '../../assets/icons/stations/outpost.svg?raw';
import megaShipSvg from '../../assets/icons/stations/mega-ship.svg?raw';

// Ingeniería (Engineering - SVG vectoriales)
import grade1Svg from '../../assets/icons/engineering/grade-1.svg?raw';
import grade2Svg from '../../assets/icons/engineering/grade-2.svg?raw';
import grade3Svg from '../../assets/icons/engineering/grade-3.svg?raw';
import grade4Svg from '../../assets/icons/engineering/grade-4.svg?raw';
import grade5Svg from '../../assets/icons/engineering/grade-5.svg?raw';
import engineerSvg from '../../assets/icons/engineering/engineer.svg?raw';
import materialSvg from '../../assets/icons/engineering/material.svg?raw';

// Iconos oficiales de navegación y módulos (Elite Dangerous EDAssets)
import explorationSvg from '../../assets/icons/navigation/exploration.svg?raw';
import combatSvg from '../../assets/icons/navigation/combat.svg?raw';
import miningSvg from '../../assets/icons/navigation/mining.svg?raw';
import tradeSvg from '../../assets/icons/navigation/trade.svg?raw';
import missionsSvg from '../../assets/icons/navigation/missions.svg?raw';
import shipSvg from '../../assets/icons/navigation/ship.svg?raw';
import dashboardSvg from '../../assets/icons/navigation/dashboard.svg?raw';
import bookmarkSvg from '../../assets/icons/navigation/bookmark.svg?raw';
import exobiologySvg from '../../assets/icons/navigation/exobiology.svg?raw';
import crimeSvg from '../../assets/icons/navigation/crime.svg?raw';
import accountsSvg from '../../assets/icons/navigation/accounts.svg?raw';

export interface PowerMeta {
  name: string;
  key: string;
  superpower: 'Empire' | 'Federation' | 'Alliance' | 'Independent';
  accentColor: string;
  glowColor: string;
  svg: string;
  portrait?: string;
}

export const POWERS: Record<string, PowerMeta> = {
  'Aisling Duval': {
    name: 'Aisling Duval',
    key: 'aisling-duval',
    superpower: 'Empire',
    accentColor: '#00d9ff',
    glowColor: 'rgba(0, 217, 255, 0.4)',
    svg: aislingDuvalSvg,
    portrait: aislingPortrait,
  },
  'Arissa Lavigny-Duval': {
    name: 'Arissa Lavigny-Duval',
    key: 'arissa-lavigny-duval',
    superpower: 'Empire',
    accentColor: '#c084fc',
    glowColor: 'rgba(192, 132, 252, 0.4)',
    svg: arissaLavignyDuvalSvg,
  },
  'Denton Patreus': {
    name: 'Denton Patreus',
    key: 'denton-patreus',
    superpower: 'Empire',
    accentColor: '#38bdf8',
    glowColor: 'rgba(56, 189, 248, 0.4)',
    svg: dentonPatreusSvg,
  },
  'Zemina Torval': {
    name: 'Zemina Torval',
    key: 'zemina-torval',
    superpower: 'Empire',
    accentColor: '#60a5fa',
    glowColor: 'rgba(96, 165, 250, 0.4)',
    svg: zeminaTorvalSvg,
  },
  'Zachary Hudson': {
    name: 'Zachary Hudson',
    key: 'zachary-hudson',
    superpower: 'Federation',
    accentColor: '#ff7700',
    glowColor: 'rgba(255, 119, 0, 0.4)',
    svg: zacharyHudsonSvg,
    portrait: zacharyPortrait,
  },
  'Felicia Winters': {
    name: 'Felicia Winters',
    key: 'felicia-winters',
    superpower: 'Federation',
    accentColor: '#ffaa00',
    glowColor: 'rgba(255, 170, 0, 0.4)',
    svg: feliciaWintersSvg,
    portrait: feliciaPortrait,
  },
  'Edmund Mahon': {
    name: 'Edmund Mahon',
    key: 'edmund-mahon',
    superpower: 'Alliance',
    accentColor: '#00e676',
    glowColor: 'rgba(0, 230, 118, 0.4)',
    svg: edmundMahonSvg,
  },
  'Li Yong-Rui': {
    name: 'Li Yong-Rui',
    key: 'li-yong-rui',
    superpower: 'Independent',
    accentColor: '#00d9ff',
    glowColor: 'rgba(0, 217, 255, 0.4)',
    svg: liYongRuiSvg,
  },
  'Archon Delaine': {
    name: 'Archon Delaine',
    key: 'archon-delaine',
    superpower: 'Independent',
    accentColor: '#ff4444',
    glowColor: 'rgba(255, 68, 68, 0.4)',
    svg: archonDelaineSvg,
  },
  'Pranav Antal': {
    name: 'Pranav Antal',
    key: 'pranav-antal',
    superpower: 'Independent',
    accentColor: '#eab308',
    glowColor: 'rgba(234, 179, 8, 0.4)',
    svg: pranavAntalSvg,
  },
  'Yuri Grom': {
    name: 'Yuri Grom',
    key: 'yuri-grom',
    superpower: 'Independent',
    accentColor: '#f97316',
    glowColor: 'rgba(249, 115, 22, 0.4)',
    svg: yuriGromSvg,
  },
};

export const SUPERPOWERS: Record<string, { name: string; color: string; svg: string }> = {
  Empire: { name: 'Imperio', color: '#38bdf8', svg: empireSvg },
  Federation: { name: 'Federación', color: '#ff7700', svg: federationSvg },
  Alliance: { name: 'Alianza', color: '#00e676', svg: allianceSvg },
};

export const MANUFACTURERS: Record<string, { name: string; svg: string }> = {
  gutamaya: { name: 'Gutamaya', svg: gutamayaSvg },
  'faulcon-delacy': { name: 'Faulcon DeLacy', svg: faulconDeLacySvg },
  'core-dynamics': { name: 'Core Dynamics', svg: coreDynamicsSvg },
  lakon: { name: 'Lakon Spaceways', svg: lakonSvg },
  'saud-kruger': { name: 'Saud Kruger', svg: saudKrugerSvg },
  'zorgon-peterson': { name: 'Zorgon Peterson', svg: zorgonPetersonSvg },
};

export const STATIONS: Record<string, { name: string; svg: string }> = {
  coriolis: { name: 'Coriolis', svg: coriolisSvg },
  orbis: { name: 'Orbis', svg: orbisSvg },
  ocellus: { name: 'Ocellus', svg: ocellusSvg },
  outpost: { name: 'Outpost', svg: outpostSvg },
  'mega-ship': { name: 'Mega-Ship', svg: megaShipSvg },
};

export const GRADES: Record<number, string> = {
  1: grade1Svg,
  2: grade2Svg,
  3: grade3Svg,
  4: grade4Svg,
  5: grade5Svg,
};

export { engineerSvg, materialSvg };

export function resolveManufacturer(shipType?: string | null): { key: string; name: string; svg: string } | null {
  if (!shipType) return null;
  const s = shipType.toLowerCase().replace(/[^a-z0-9]/g, '');

  // Gutamaya
  if (s.includes('cutter') || s.includes('clipper') || s.includes('courier') || s.includes('imperial') || s.includes('empiretrader') || s.includes('corsair')) {
    return { key: 'gutamaya', ...MANUFACTURERS['gutamaya'] };
  }

  // Core Dynamics
  if (s.includes('corvette') || s.includes('gunship') || s.includes('dropship') || s.includes('assaultship') || s.includes('vulture') || s.includes('eagle') || s.includes('kestrel')) {
    return { key: 'core-dynamics', ...MANUFACTURERS['core-dynamics'] };
  }

  // Saud Kruger
  if (s.includes('beluga') || s.includes('orca') || s.includes('dolphin')) {
    return { key: 'saud-kruger', ...MANUFACTURERS['saud-kruger'] };
  }

  // Zorgon Peterson
  if (s.includes('ferdelance') || s.includes('fdl') || s.includes('mamba') || s.includes('hauler') || s.includes('adder') || s.includes('mandalay') || s.includes('panther')) {
    return { key: 'zorgon-peterson', ...MANUFACTURERS['zorgon-peterson'] };
  }

  // Lakon
  if (s.includes('type6') || s.includes('type7') || s.includes('type8') || s.includes('type9') || s.includes('type10') || s.includes('type11') ||
      s.includes('asp') || s.includes('diamondback') || s.includes('keelback') || s.includes('independanttrader') || s.includes('independenttrader') ||
      s.includes('chieftain') || s.includes('crusader') || s.includes('challenger') || s.includes('typex')) {
    return { key: 'lakon', ...MANUFACTURERS['lakon'] };
  }

  // Faulcon DeLacy (por defecto y para anaconda, python, cobra, krait, sidewinder, viper)
  if (s.includes('anaconda') || s.includes('python') || s.includes('cobra') || s.includes('krait') || s.includes('sidewinder') || s.includes('viper')) {
    return { key: 'faulcon-delacy', ...MANUFACTURERS['faulcon-delacy'] };
  }

  return { key: 'faulcon-delacy', ...MANUFACTURERS['faulcon-delacy'] };
}

export function resolveStation(stationType?: string | null): { key: string; name: string; svg: string } {
  if (!stationType) return { key: 'coriolis', ...STATIONS['coriolis'] };
  const s = stationType.toLowerCase();
  if (s.includes('orbis')) return { key: 'orbis', ...STATIONS['orbis'] };
  if (s.includes('ocellus')) return { key: 'ocellus', ...STATIONS['ocellus'] };
  if (s.includes('outpost')) return { key: 'outpost', ...STATIONS['outpost'] };
  if (s.includes('mega') || s.includes('fleetcarrier') || s.includes('carrier')) return { key: 'mega-ship', ...STATIONS['mega-ship'] };
  return { key: 'coriolis', ...STATIONS['coriolis'] };
}

export const NAV_ICONS: Record<string, string> = {
  exploration: explorationSvg,
  combat: combatSvg,
  mining: miningSvg,
  trade: tradeSvg,
  missions: missionsSvg,
  engineering: engineerSvg,
  carrier: megaShipSvg,
  colonisation: outpostSvg,
  powerplay: aislingDuvalSvg,
  ship: shipSvg,
  dashboard: dashboardSvg,
  navigation: bookmarkSvg,
  exobiology: exobiologySvg,
  bgs: federationSvg,
  accounts: accountsSvg,
  crime: crimeSvg,
};

export function resolveNavIcon(id: string): string | null {
  return NAV_ICONS[id] ?? null;
}
