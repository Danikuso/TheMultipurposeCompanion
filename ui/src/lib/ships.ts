/**
 * Canonical ship names and formatting for Elite Dangerous.
 * Maps internal journal / CAPI identifiers to proper display names.
 */

export const CANONICAL_SHIPS: Record<string, string> = {
  // Small Ships
  sidewinder: 'Sidewinder Mk I',
  eagle: 'Eagle Mk II',
  hauler: 'Hauler',
  adder: 'Adder',
  viper: 'Viper Mk III',
  cobramkiii: 'Cobra Mk III',
  viper_mkiv: 'Viper Mk IV',
  diamondback: 'Diamondback Scout',
  cobramkiv: 'Cobra Mk IV',
  diamondbackxl: 'Diamondback Explorer',
  empire_eagle: 'Imperial Eagle',
  empire_courier: 'Imperial Courier',
  dolphin: 'Dolphin',
  vulture: 'Vulture',
  kestrel: 'Kestrel Mk II',
  kestrel_mkii: 'Kestrel Mk II',
  cobramkv: 'Cobra Mk V',

  // Medium Ships
  asp: 'Asp Explorer',
  asp_scout: 'Asp Scout',
  independant_trader: 'Keelback',
  type6: 'Type-6 Transporter',
  typex: 'Alliance Chieftain',
  typex_2: 'Alliance Crusader',
  typex_3: 'Alliance Challenger',
  federation_dropship: 'Federal Dropship',
  federation_dropship_mkii: 'Federal Assault Ship',
  federation_gunship: 'Federal Gunship',
  python: 'Python',
  python_nx: 'Python Mk II',
  ferdelance: 'Fer-de-Lance',
  mamba: 'Mamba',
  krait_mkii: 'Krait Mk II',
  krait_light: 'Krait Phantom',
  mandalay: 'Mandalay',
  corsair: 'Corsair',

  // Large Ships
  type7: 'Type-7 Transporter',
  type8: 'Type-8 Transporter',
  type9: 'Type-9 Heavy',
  type9_military: 'Type-10 Defender',
  type11: 'Type-11 Prospector',
  empire_trader: 'Imperial Clipper',
  orca: 'Orca',
  belugaliner: 'Beluga Liner',
  federation_corvette: 'Federal Corvette',
  cutter: 'Imperial Cutter',
  anaconda: 'Anaconda',
  explorer_nx: 'Caspian Explorer',
  panthermkii: 'Panther Clipper Mk II',

  // Fighters (SLF)
  gu97: 'GU-97 Imperial Fighter',
  empire_fighter: 'GU-97 Imperial Fighter',
  independent_fighter: 'Taipan Fighter',
  taipan: 'Taipan Fighter',
  federation_fighter: 'F63 Condor',
  condor: 'F63 Condor',
  gdn_hybrid_v1: 'Trident (Guardian Fighter)',
  gdn_hybrid_v2: 'Javelin (Guardian Fighter)',
  gdn_hybrid_v3: 'Lance (Guardian Fighter)',

  // SRVs
  testbuggy: 'Scarab SRV',
  combatbuggy: 'Scorpion SRV',
  srv: 'SRV',

  // Megaships & Carriers
  fleetcarrier: 'Fleet Carrier',
};

/** Normalize string by stripping non-alphanumeric chars and converting to lowercase. */
export function normalizeShipKey(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]/g, '');
}

/** Pre-indexed map with normalized keys for O(1) matching regardless of casing, underscores, or hyphens. */
const NORMALIZED_SHIPS: Record<string, string> = {};

// Index standard entries
for (const [key, name] of Object.entries(CANONICAL_SHIPS)) {
  NORMALIZED_SHIPS[normalizeShipKey(key)] = name;
  NORMALIZED_SHIPS[normalizeShipKey(name)] = name;
}

// Common aliases and alternate spellings
const ALIASES: Record<string, string> = {
  // Keelback
  independenttrader: 'Keelback',
  keelback: 'Keelback',

  // Python Mk II
  pythonmkii: 'Python Mk II',
  pythonmk2: 'Python Mk II',

  // Krait Phantom
  kraitphantom: 'Krait Phantom',

  // Krait Mk II
  kraitmk2: 'Krait Mk II',

  // Type-10
  type10: 'Type-10 Defender',
  type10defender: 'Type-10 Defender',

  // Diamondback Explorer / Scout
  diamondbackexplorer: 'Diamondback Explorer',
  diamondbackscout: 'Diamondback Scout',

  // Imperial ships
  imperialclipper: 'Imperial Clipper',
  clipper: 'Imperial Clipper',
  imperialcourier: 'Imperial Courier',
  courier: 'Imperial Courier',
  imperialeagle: 'Imperial Eagle',
  imperialcutter: 'Imperial Cutter',

  // Federal ships
  federalcorvette: 'Federal Corvette',
  corvette: 'Federal Corvette',
  federaldropship: 'Federal Dropship',
  federalassaultship: 'Federal Assault Ship',
  federalgunship: 'Federal Gunship',

  // Alliance ships
  alliancechieftain: 'Alliance Chieftain',
  chieftain: 'Alliance Chieftain',
  alliancecrusader: 'Alliance Crusader',
  crusader: 'Alliance Crusader',
  alliancechallenger: 'Alliance Challenger',
  challenger: 'Alliance Challenger',

  // Beluga
  beluga: 'Beluga Liner',

  // Fer-de-Lance
  fdl: 'Fer-de-Lance',
  ferdelance: 'Fer-de-Lance',

  // Cobras & Vipers
  cobramk3: 'Cobra Mk III',
  cobramk4: 'Cobra Mk IV',
  cobramk5: 'Cobra Mk V',
  vipermk3: 'Viper Mk III',
  vipermk4: 'Viper Mk IV',
  eaglemk2: 'Eagle Mk II',
  sidewindermk1: 'Sidewinder Mk I',

  // Modern / New Ships
  caspian: 'Caspian Explorer',
  caspianexplorer: 'Caspian Explorer',
  pantherclipper: 'Panther Clipper Mk II',
  pantherclippermkii: 'Panther Clipper Mk II',
  type11prospector: 'Type-11 Prospector',
};

for (const [alias, name] of Object.entries(ALIASES)) {
  NORMALIZED_SHIPS[normalizeShipKey(alias)] = name;
}

/** Fallback cleanly formats unknown ship names into title case with proper Mark/Type numerals. */
export function fallbackShipFormat(raw: string): string {
  // Strip leading '$' or trailing ';' and common internal tokens like '_name'
  let s = raw.replace(/^\$|\;$/g, '').replace(/_name$/i, '').trim();

  // Insert space before CamelCase transitions: e.g. CobraMkIII -> Cobra Mk III
  s = s.replace(/([a-z])([A-Z])/g, '$1 $2');

  // Replace underscores and hyphens with spaces
  s = s.replace(/[_\-]+/g, ' ');

  // Normalize Mark Roman numerals: e.g. "mk iii", "mk 3", "mk3" -> "Mk III"
  s = s.replace(/\bmk\s*([0-9ivx]+)\b/gi, (_, m: string) => {
    const num = m.toLowerCase();
    const roman =
      num === '1' || num === 'i'
        ? 'I'
        : num === '2' || num === 'ii'
          ? 'II'
          : num === '3' || num === 'iii'
            ? 'III'
            : num === '4' || num === 'iv'
              ? 'IV'
              : num === '5' || num === 'v'
                ? 'V'
                : m.toUpperCase();
    return `Mk ${roman}`;
  });

  // Handle Type-X hyphenation
  s = s.replace(/\btype\s*(\d+)\b/gi, 'Type-$1');

  // Title case words
  return s
    .split(/\s+/)
    .map((w) => {
      if (/^mk$/i.test(w)) return 'Mk';
      if (/^[ivx]+$/i.test(w)) return w.toUpperCase();
      if (/^type-\d+$/i.test(w)) return `Type-${w.split('-')[1]}`;
      return w.charAt(0).toUpperCase() + w.slice(1).toLowerCase();
    })
    .join(' ');
}

/**
 * Format any ship model into its proper canonical display name.
 * Handles missing/null values, raw journal identifiers, CAPI strings, and unlocalized $ tokens.
 */
export function formatShipName(raw?: string | null, localised?: string | null): string {
  // If localised is a valid human-readable string (not a raw token like $ADDER_NAME;)
  if (localised && localised.trim().length > 0 && !localised.startsWith('$') && !localised.endsWith(';')) {
    const locTrim = localised.trim();
    const rawTrim = (raw ?? '').trim();
    // If localised is already properly capitalized and not just an identical all-lowercase identifier
    if (locTrim.toLowerCase() !== rawTrim.toLowerCase() || locTrim !== locTrim.toLowerCase()) {
      return locTrim;
    }
  }

  const key = (raw ?? '').trim();
  if (!key) {
    if (localised && !localised.startsWith('$') && !localised.endsWith(';')) {
      return localised.trim();
    }
    return '—';
  }

  // Look up in normalized canonical dictionary
  const norm = normalizeShipKey(key);
  if (NORMALIZED_SHIPS[norm]) {
    return NORMALIZED_SHIPS[norm];
  }

  return fallbackShipFormat(key);
}
