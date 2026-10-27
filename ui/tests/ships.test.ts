import { describe, expect, it } from 'vitest';
import { formatShipName, fallbackShipFormat, normalizeShipKey } from '../src/lib/ships';

describe('ships', () => {
  it('normalizes ship keys', () => {
    expect(normalizeShipKey('Cobra Mk III')).toBe('cobramkiii');
    expect(normalizeShipKey('type-9_military')).toBe('type9military');
    expect(normalizeShipKey('Mandalay')).toBe('mandalay');
  });

  it('formats canonical ships correctly', () => {
    expect(formatShipName('mandalay')).toBe('Mandalay');
    expect(formatShipName('mandalay', null)).toBe('Mandalay');
    expect(formatShipName('cobramkiii')).toBe('Cobra Mk III');
    expect(formatShipName('diamondbackxl')).toBe('Diamondback Explorer');
    expect(formatShipName('independant_trader')).toBe('Keelback');
    expect(formatShipName('independent_trader')).toBe('Keelback');
    expect(formatShipName('type9_military')).toBe('Type-10 Defender');
    expect(formatShipName('python_nx')).toBe('Python Mk II');
    expect(formatShipName('krait_light')).toBe('Krait Phantom');
    expect(formatShipName('federation_corvette')).toBe('Federal Corvette');
    expect(formatShipName('cutter')).toBe('Imperial Cutter');
    expect(formatShipName('empire_trader')).toBe('Imperial Clipper');
    expect(formatShipName('empire_courier')).toBe('Imperial Courier');
    expect(formatShipName('typex')).toBe('Alliance Chieftain');
    expect(formatShipName('typex_2')).toBe('Alliance Crusader');
    expect(formatShipName('typex_3')).toBe('Alliance Challenger');
    expect(formatShipName('type6')).toBe('Type-6 Transporter');
    expect(formatShipName('MediumTransport01')).toBe('Type-6 Transporter');
    expect(formatShipName('mediumtransport01')).toBe('Type-6 Transporter');
    expect(formatShipName('MediumTransport02')).toBe('Keelback');
    expect(formatShipName('viper')).toBe('Viper Mk III');
    expect(formatShipName('Viper')).toBe('Viper Mk III');
    expect(formatShipName('Python_NX')).toBe('Python Mk II');
    expect(formatShipName('type7')).toBe('Type-7 Transporter');
    expect(formatShipName('type8')).toBe('Type-8 Transporter');
    expect(formatShipName('type9')).toBe('Type-9 Heavy');
    expect(formatShipName('type11')).toBe('Type-11 Prospector');
    expect(formatShipName('ferdelance')).toBe('Fer-de-Lance');
    expect(formatShipName('belugaliner')).toBe('Beluga Liner');
    expect(formatShipName('vulture')).toBe('Vulture');
    expect(formatShipName('kestrel_mkii')).toBe('Kestrel Mk II');
    expect(formatShipName('corsair')).toBe('Corsair');
  });

  it('filters out raw unlocalized $ tokens and resolves canonical name', () => {
    expect(formatShipName('mandalay', '$MANDALAY_NAME;')).toBe('Mandalay');
    expect(formatShipName('adder', '$ADDER_NAME;')).toBe('Adder');
    expect(formatShipName('cobramkiii', '$COBRAMKIII_NAME;')).toBe('Cobra Mk III');
    expect(formatShipName('explorer_nx', '$CASPIAN_NAME;')).toBe('Caspian Explorer');
  });

  it('respects clean human localized names', () => {
    expect(formatShipName('anaconda', 'Anaconda')).toBe('Anaconda');
    expect(formatShipName('mandalay', 'Mandalay')).toBe('Mandalay');
  });

  it('handles null, undefined, or empty values cleanly', () => {
    expect(formatShipName(null, null)).toBe('—');
    expect(formatShipName(undefined, undefined)).toBe('—');
    expect(formatShipName('', '')).toBe('—');
  });

  it('formats unknown ships using fallback title case and roman numerals', () => {
    expect(fallbackShipFormat('experimental_explorer_mk2')).toBe('Experimental Explorer Mk II');
    expect(fallbackShipFormat('some_new_hauler')).toBe('Some New Hauler');
    expect(fallbackShipFormat('type 12')).toBe('Type-12');
  });
});
