import { FluentBundle, FluentResource } from '@fluent/bundle';
import { localeRef } from './locale.svelte';

const rawFiles = import.meta.glob<string>('../../../locales/*/main.ftl', {
  query: '?raw',
  eager: true,
  import: 'default',
});

const bundles = new Map<string, FluentBundle>();
const keysMap: Record<string, string[]> = {};

for (const [filePath, content] of Object.entries(rawFiles)) {
  const match = filePath.match(/locales\/([^/]+)\/main\.ftl$/);
  if (!match) continue;
  const lang = match[1];
  const resource = new FluentResource(content);
  const bundle = new FluentBundle(lang, { useIsolating: false });
  bundle.addResource(resource);
  bundles.set(lang, bundle);

  const keys: string[] = [];
  for (const entry of resource.body) {
    if ('id' in entry && typeof entry.id === 'string' && !entry.id.startsWith('-')) {
      keys.push(entry.id);
    }
  }
  keys.sort();
  keysMap[lang] = keys;
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
