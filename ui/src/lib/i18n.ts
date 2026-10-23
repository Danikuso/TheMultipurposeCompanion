import { FluentBundle, FluentResource } from '@fluent/bundle';
import { localeRef } from './locale.svelte';

const rawFiles = import.meta.glob<string>('../../../locales/*/*.ftl', {
  query: '?raw',
  eager: true,
  import: 'default',
});

const bundles = new Map<string, FluentBundle>();
const keysMap: Record<string, string[]> = {};

const keySets: Record<string, Set<string>> = {};

for (const filePath of Object.keys(rawFiles).sort()) {
  const match = filePath.match(/locales\/([^/]+)\/([^/]+)\.ftl$/);
  if (!match) continue;
  const lang = match[1];
  const resource = new FluentResource(rawFiles[filePath]);
  let bundle = bundles.get(lang);
  if (!bundle) {
    bundle = new FluentBundle(lang, { useIsolating: false });
    bundles.set(lang, bundle);
    keySets[lang] = new Set();
  }
  const errors = bundle.addResource(resource);
  if (errors.length > 0) {
    throw new Error(`${filePath}: ${errors.map((e) => e.message).join('; ')}`);
  }
  for (const entry of resource.body) {
    if ('id' in entry && typeof entry.id === 'string' && !entry.id.startsWith('-')) {
      keySets[lang].add(entry.id);
    }
  }
}
for (const [lang, set] of Object.entries(keySets)) {
  keysMap[lang] = [...set].sort();
}

export function setLocale(lang: string): void {
  localeRef.current = bundles.has(lang) ? lang : 'es';
}

export function getLocale(): string {
  return localeRef.current;
}

export function t(key: string, args?: Record<string, string | number>): string {
  const lang = localeRef.current;
  const primary = bundles.get(lang) ?? bundles.get('es');
  const fallback = bundles.get('es');

  for (const bundle of [primary, fallback]) {
    if (!bundle) continue;
    const msg = bundle.getMessage(key);
    if (msg && msg.value) {
      return bundle.formatPattern(msg.value, args);
    }
  }
  return key;
}

export function localeKeys(): Record<string, string[]> {
  return keysMap;
}

/** Whether the current translation explicitly awaits confirmation. */
export function translationPending(key: string): boolean {
  const primary = bundles.get(getLocale());
  const fallback = bundles.get('es');
  for (const bundle of [primary, fallback]) {
    const message = bundle?.getMessage(key);
    if (message?.value) {
      const pending = message.attributes.pending;
      return pending !== undefined && bundle!.formatPattern(pending).trim() === '1';
    }
  }
  return false;
}
