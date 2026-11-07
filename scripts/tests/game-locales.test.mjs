import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { gameRows, languages, source, normalize, keyFor, localizeGameResources, syncGameLocales } from '../sync-game-locales.mjs';

test('all source entries have a complete, nonempty label in each supported language', () => {
  const reference = gameRows('es').map(row => row.key).sort();
  assert.equal(reference.length, new Set(reference).size);
  for (const lang of languages) {
    const rows = gameRows(lang);
    assert.deepEqual(rows.map(row => row.key).sort(), reference);
    assert.ok(rows.every(row => row.label.trim() && !/\.pending/.test(row.label)));
  }
  const index = JSON.parse(readFileSync('crates/tmc-i18n/data/game-label-index.json', 'utf8'));
  for (const [kind, rows] of Object.entries(source.groups)) for (const row of rows) {
    assert.equal(index[kind][normalize(row.id)].key, keyFor(kind, row.id));
  }
});

test('engineering guide resource aliases reference existing labels, including plurals', () => {
  const keys = new Set(gameRows('en').map(row => row.key));
  const text = localizeGameResources('Sensor Fragments, Occupied Escape Pods, Xihe Companions, Insight Entertainment Suites, Digital Designs, Push.');
  const refs = [...text.matchAll(/\{ ([\w-]+) \}/g)].map(match => match[1]);
  assert.equal(refs.length, 6);
  assert.ok(refs.every(key => keys.has(key)));
  assert.equal(localizeGameResources('Give 25 Sensor Fragments.'), 'Give 25 × { game-material-unknownenergysource }.');
});

test('checked-in resources and editorial corrections reproduce without changes', () => {
  assert.equal(syncGameLocales(true), 0);
});
