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
  if (s.startsWith('hugehardpoint')) return `Anclaje Enorme ${slot.replace(/\D/g, '') || '1'}`;
  if (s.startsWith('largehardpoint')) return `Anclaje Grande ${slot.replace(/\D/g, '') || '1'}`;
  if (s.startsWith('mediumhardpoint')) return `Anclaje Mediano ${slot.replace(/\D/g, '') || '1'}`;
  if (s.startsWith('smallhardpoint')) return `Anclaje Pequeño ${slot.replace(/\D/g, '') || '1'}`;
  if (s.startsWith('tinyhardpoint')) return `Soporte Utilitario ${slot.replace(/\D/g, '') || '1'}`;
  if (s === 'armour') return 'Blindaje del Casco';
  if (s === 'powerplant') return 'Central de Energía';
  if (s === 'mainengines') return 'Motores (Impulsores)';
  if (s === 'frameshiftdrive') return 'Motor de Distorsión (FSD)';
  if (s === 'lifesupport') return 'Soporte Vital';
  if (s === 'powerdistributor') return 'Distribuidor de Energía';
  if (s === 'radar') return 'Sensores (Radar)';
  if (s === 'fueltank') return 'Depósito de Combustible';
  if (s === 'planetaryapproachsuite') return 'Suite de Aproximación Planetaria';
  if (s === 'cargohatch') return 'Compuerta de Carga';
  if (s.startsWith('military')) return `Refuerzo Militar ${slot.replace(/\D/g, '') || '1'}`;
  if (s.startsWith('slot')) {
    const parts = slot.split('_');
    const slotNum = parts[0]?.replace(/\D/g, '') || '';
    const size = parts[1]?.replace(/\D/g, '') || '';
    return `Ranura ${slotNum}${size ? ` (Talla ${size})` : ''}`;
  }
  return slot;
}

const CLASS_LETTERS: Record<number, string> = {
  1: 'E',
  2: 'D',
  3: 'C',
  4: 'B',
  5: 'A',
};

export function formatItemName(rawItem: string): { label: string; sizeClass: string } {
  if (!rawItem) return { label: '—', sizeClass: '' };
  const lower = rawItem.toLowerCase();

  // Extraer talla y clase numérica (sizeX_classY)
  const sizeMatch = lower.match(/size(\d+)/);
  const classMatch = lower.match(/class(\d+)/);
  const size = sizeMatch ? sizeMatch[1] : '';
  const classNum = classMatch ? parseInt(classMatch[1], 10) : null;
  const classLetter = classNum ? (CLASS_LETTERS[classNum] ?? String(classNum)) : '';
  const sizeClass = size && classLetter ? `${size}${classLetter}` : size || classLetter;

  // Tipos comunes
  if (lower.includes('hyperdrive')) return { label: 'Motor de Distorsión (FSD)', sizeClass };
  if (lower.includes('powerplant')) return { label: 'Central de Energía', sizeClass };
  if (lower.includes('engine')) return { label: 'Impulsores', sizeClass };
  if (lower.includes('lifesupport')) return { label: 'Soporte Vital', sizeClass };
  if (lower.includes('powerdistributor')) return { label: 'Distribuidor de Energía', sizeClass };
  if (lower.includes('sensors') || lower.includes('radar')) return { label: 'Sensores', sizeClass };
  if (lower.includes('fueltank')) return { label: 'Depósito de Combustible', sizeClass };
  if (lower.includes('cargorack')) {
    const tons = size ? Math.pow(2, parseInt(size, 10)) : '';
    return { label: `Bodega de Carga${tons ? ` (${tons} t)` : ''}`, sizeClass };
  }
  if (lower.includes('shieldgenerator')) return { label: 'Generador de Escudos', sizeClass };
  if (lower.includes('fuelscoop')) return { label: 'Recolector de Combustible (Fuel Scoop)', sizeClass };
  if (lower.includes('guardianfsdbooster')) return { label: 'Potenciador FSD de Guardian', sizeClass };
  if (lower.includes('detailedsurfacescanner')) return { label: 'Escáner de Superficie Detallado (DSS)', sizeClass: '1I' };
  if (lower.includes('shieldbooster')) return { label: 'Potenciador de Escudos (Booster)', sizeClass };
  if (lower.includes('chafflauncher')) return { label: 'Lanzador de Señuelos (Chaff)', sizeClass: '0I' };
  if (lower.includes('heatsinklauncher')) return { label: 'Disipador Térmico (Heat Sink)', sizeClass: '0I' };
  if (lower.includes('pointdefence')) return { label: 'Defensa de Punto (Point Defence)', sizeClass: '0I' };
  if (lower.includes('electroniccountermeasure')) return { label: 'Contramedidas Electrónicas (ECM)', sizeClass: '0I' };
  if (lower.includes('shieldcellbank')) return { label: 'Célula de Recarga de Escudos (SCB)', sizeClass };
  if (lower.includes('hullreinforcement')) return { label: 'Refuerzo de Casco (HRP)', sizeClass };
  if (lower.includes('modulereinforcement')) return { label: 'Refuerzo de Módulos (MRP)', sizeClass };
  if (lower.includes('repairer')) return { label: 'Unidad AFMU de Mantenimiento', sizeClass };
  if (lower.includes('refinery')) return { label: 'Refinería', sizeClass };
  if (lower.includes('multicannon')) {
    const mount = lower.includes('turret') ? 'Torreta' : lower.includes('gimbal') ? 'Cardanado' : 'Fijo';
    return { label: `Multicañón (${mount})`, sizeClass };
  }
  if (lower.includes('beamlaser')) {
    const mount = lower.includes('turret') ? 'Torreta' : lower.includes('gimbal') ? 'Cardanado' : 'Fijo';
    return { label: `Láser de Haz (${mount})`, sizeClass };
  }
  if (lower.includes('pulselaser')) {
    const mount = lower.includes('turret') ? 'Torreta' : lower.includes('gimbal') ? 'Cardanado' : 'Fijo';
    return { label: `Láser de Pulsos (${mount})`, sizeClass };
  }
  if (lower.includes('burstlaser')) {
    const mount = lower.includes('turret') ? 'Torreta' : lower.includes('gimbal') ? 'Cardanado' : 'Fijo';
    return { label: `Láser de Ráfagas (${mount})`, sizeClass };
  }
  if (lower.includes('plasmaaccelerator')) return { label: 'Acelerador de Plasma', sizeClass };
  if (lower.includes('cannon')) return { label: 'Cañón', sizeClass };
  if (lower.includes('fragmentcannon')) return { label: 'Cañón de Fragmentación', sizeClass };
  if (lower.includes('railgun')) return { label: 'Cañón de Riel (Railgun)', sizeClass };
  if (lower.includes('missile')) return { label: 'Lanzador de Misiles', sizeClass };
  if (lower.includes('torpedo')) return { label: 'Pilón de Torpedos', sizeClass };
  if (lower.includes('mininglaser')) return { label: 'Láser de Minería', sizeClass };
  if (lower.includes('subsurfacemining')) return { label: 'Taladro Subsuperficial', sizeClass };
  if (lower.includes('seismiccharge')) return { label: 'Lanzador de Cargas Sísmicas', sizeClass };
  if (lower.includes('abrasionblaster')) return { label: 'Bláster de Abrasión', sizeClass };
  if (lower.includes('dronecontrol_collection')) return { label: 'Controlador de Drones Recolectores', sizeClass };
  if (lower.includes('dronecontrol_prospector')) return { label: 'Controlador de Drones de Prospección', sizeClass };
  if (lower.includes('dronecontrol_repair')) return { label: 'Controlador de Drones de Reparación', sizeClass };
  if (lower.includes('buggybay')) return { label: 'Hangar de Vehículos Planetarios (SRV)', sizeClass };
  if (lower.includes('fighterbay')) return { label: 'Hangar de Cazas SLF', sizeClass };
  if (lower.includes('dockingcomputer')) return { label: 'Ordenador de Atraque', sizeClass: '1E' };
  if (lower.includes('supercruiseassist')) return { label: 'Asistente de Supercrucero', sizeClass: '1E' };
  if (lower.includes('passengercabin')) return { label: 'Cabinas de Pasajeros', sizeClass };
  if (lower.includes('planetapproachsuite')) return { label: 'Suite Planetaria', sizeClass: '1I' };
  if (lower.includes('armour_grade1') || lower.includes('armour_grade2') || lower.includes('armour_grade3')) {
    if (lower.includes('grade1')) return { label: 'Blindaje Compuesto Ligero', sizeClass: '1I' };
    if (lower.includes('grade2')) return { label: 'Blindaje Compuesto Reforzado', sizeClass: '1I' };
    if (lower.includes('grade3')) return { label: 'Blindaje Militar Compuesto', sizeClass: '1I' };
  }

  // Fallback limpio
  const clean = rawItem.replace(/^(int_|hpt_)/, '').replace(/_/g, ' ');
  return { label: clean.charAt(0).toUpperCase() + clean.slice(1), sizeClass };
}
