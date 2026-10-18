import { describe, expect, test } from 'vitest';
import { localeKeys, setLocale, t } from '../src/lib/i18n';

describe('i18n', () => {
  test('t falls back to es and formats args', () => {
    setLocale('xx');
    const msg = t('alert-hull-damage', { pct: 25 });
    expect(msg).toContain('25');
    expect(msg).not.toBe('alert-hull-damage');
  });

  test('loads keys from every ftl file', () => {
    setLocale('en');
    expect(t('panel-system-bodies')).toBe('System bodies');
    expect(t('panel-route-strip')).toBe('Route');
    expect(t('panel-exobiology')).toBe('Exobiology');
    setLocale('de');
    expect(t('panel-system-bodies')).toBe('Systemkörper');
    setLocale('es');
  });

  test('all locales define the same keys', () => {
    const keysByLang = localeKeys();
    const langs = Object.keys(keysByLang).sort();
    expect(langs).toEqual(['de', 'en', 'es', 'fr', 'pt']);
    const ref = keysByLang['es'];
    expect(ref.length).toBeGreaterThanOrEqual(67);
    for (const lang of langs) {
      expect(keysByLang[lang]).toEqual(ref);
    }
  });
});
