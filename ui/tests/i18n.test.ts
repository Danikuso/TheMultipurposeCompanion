import { describe, expect, test } from 'vitest';
import { FluentBundle, FluentResource } from '@fluent/bundle';
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

  test('French and Portuguese plural forms preserve zero and other quantities', () => {
    try {
      for (const lang of ['fr', 'pt']) {
        setLocale(lang);
        for (const count of [0, 1, 2]) {
          for (const key of ['exobiology-signals', 'mining-hotspots-count']) {
            expect(t(key, { count }), `${lang}:${key}:${count}`).toContain(String(count));
          }
          expect(t('missions-massacre-missing', { kills: count }), `${lang}:kills:${count}`).toContain(String(count));
        }
      }
      setLocale('pt');
      expect(t('mining-limpets-left', { count: 0 })).toBe('restam 0 drones');
      expect(t('exobiology-signals', { count: 0 })).toBe('0 sinais biológicos');
    } finally { setLocale('es'); }
  });

  test('all resources parse and preserve arguments across languages without unresolved references', () => {
    const files = import.meta.glob<string>('../../locales/*/*.ftl', { query: '?raw', eager: true, import: 'default' });
    const variables = (value: unknown): string[] => {
      const names = new Set<string>();
      const visit = (node: unknown) => {
        if (!node || typeof node !== 'object') return;
        const entry = node as Record<string, unknown>;
        if (entry.type === 'var' && typeof entry.name === 'string') names.add(entry.name);
        for (const child of Object.values(entry)) {
          if (Array.isArray(child)) child.forEach(visit); else visit(child);
        }
      };
      visit(value); return [...names].sort();
    };
    const byLanguage = new Map<string, Map<string, string[]>>();
    for (const lang of ['es', 'en', 'fr', 'de', 'pt']) {
      const bundle = new FluentBundle(lang, { useIsolating: false });
      const messages = new Map<string, string[]>(); byLanguage.set(lang, messages);
      for (const [path, text] of Object.entries(files).filter(([path]) => path.includes(`/locales/${lang}/`))) {
        const resource = new FluentResource(text);
        expect(resource.body.length, path).toBe([...text.matchAll(/^-?[a-zA-Z][\w-]*\s*=/gm)].length);
        expect(bundle.addResource(resource), path).toEqual([]);
        for (const entry of resource.body) messages.set(entry.id, variables(entry));
      }
      expect(messages.size).toBeGreaterThan(1900);
      // Supply every declared variable so errors identify broken references or patterns.
      const args = Object.fromEntries([...messages.values()].flat().map(name => [name, 47]));
      for (const key of messages.keys()) {
        const message = bundle.getMessage(key)!;
        const errors: Error[] = [];
        if (message.value) bundle.formatPattern(message.value, args, errors);
        for (const pattern of Object.values(message.attributes)) bundle.formatPattern(pattern, args, errors);
        expect(errors, `${lang}:${key}`).toEqual([]);
      }
    }
    for (const [lang, messages] of byLanguage) {
      for (const [key, args] of byLanguage.get('es')!) expect(messages.get(key), `${lang}:${key}`).toEqual(args);
    }
  });

  test('the engineer guide uses the same localized ranks as the commander panels', () => {
    try {
      for (const lang of ['es', 'en', 'fr', 'de', 'pt']) {
        setLocale(lang);
        expect(t('unlock-felicity-farseer')).toContain(t('explore-rank-2'));
        expect(t('unlock-the-sarge')).toContain(t('federation-rank-3'));
      }
      setLocale('es');
      expect(t('exploration-mark-scoopable')).toBe('Repostable');
      expect(t('strip-scoopable')).toBe('Repostable');
    } finally { setLocale('es'); }
  });
});
