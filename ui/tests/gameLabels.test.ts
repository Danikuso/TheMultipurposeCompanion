import { afterEach, expect, test } from 'vitest';
import { setLocale } from '../src/lib/i18n';
import { gameLabel, resolveGameLabel } from '../src/lib/gameLabels';

afterEach(() => setLocale('es'));

test('known game resources follow the app language rather than the journal language', () => {
  for (const [lang, water, carbon] of [
    ['es', 'Agua', 'Carbono'], ['en', 'Water', 'Carbon'], ['fr', 'Eau', 'Carbone'],
    ['de', 'Wasser', 'Kohlenstoff'], ['pt', 'Água', 'Carbono'],
  ]) {
    setLocale(lang);
    expect(gameLabel('commodity', 'water', 'Water from an English journal')).toBe(water);
    expect(gameLabel('material', 'carbon', 'Carbon from an English journal')).toBe(carbon);
  }
});

test('metadata symbols, spelling variants and unknown future values keep their identity', () => {
  setLocale('es');
  expect(gameLabel('government', '$government_Democracy;')).toBe('Democracia');
  expect(gameLabel('state', 'Civil War')).toBe(gameLabel('state', 'CivilWar'));
  expect(gameLabel('superpower', 'Federation')).toBe('Federación');
  expect(gameLabel('commodity', '$Gold_Name;')).toBe('Oro');
  expect(gameLabel('security', 'High')).toBe(gameLabel('security', '$SYSTEM_SECURITY_high;'));
  expect(gameLabel('commodity', 'future_cargo', 'Future Cargo')).toBe('Future Cargo');
  expect(gameLabel('commodity', 'future_cargo')).toBe('future_cargo');
  expect(gameLabel('commodity', null)).toBe('—');
  expect(gameLabel('commodity', 'constructor')).toBe('constructor');
  expect(gameLabel('government', ' $government_Democracy; ')).toBe('Democracia');
});

test('localized search names resolve to game IDs without translating unknown future names', () => {
  setLocale('fr');
  expect(resolveGameLabel('commodity', 'Eau')?.id.toLowerCase()).toBe('water');
  expect(resolveGameLabel('commodity', 'Água')?.id.toLowerCase()).toBe('water');
  expect(resolveGameLabel('commodity', 'Unknown Brand')).toBeUndefined();
  expect(gameLabel('commodity', 'ChemicalWaste')).not.toBe(gameLabel('material', 'chemicaldistillery'));
});
