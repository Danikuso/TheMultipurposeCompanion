import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const entries = JSON.parse(readFileSync(new URL('../../crates/tmc-modules/data/engineers.json', import.meta.url))).entries;
test('all 38 engineer guides have concrete translated instructions and valid acyclic contacts', () => {
  assert.equal(entries.length, 38);
  const byId = new Map(entries.map(e => [e.id, e]));
  const languages = ['es', 'en', 'fr', 'de', 'pt'].map(lang => readFileSync(new URL(`../../locales/${lang}/engineering-data.ftl`, import.meta.url), 'utf8'));
  for (const e of entries) {
    assert.equal(e.unlock.guide_verified, true, e.name);
    assert.ok(e.unlock.guide_sources.length, e.name);
    for (const key of [e.unlock.discovery_key, e.unlock.requirement_key, e.unlock.invite_key, e.unlock.reputation_key]) {
      assert.ok(key, e.name);
      for (const text of languages) {
        const value = text.match(new RegExp(`^${key} = (.+)$`, 'm'))?.[1];
        assert.ok(value && value.length > 15, `${e.name}: ${key}`);
        assert.notEqual(value, `Cumple el requisito de desbloqueo de ${e.name}.`);
        assert.notEqual(value, `Consigue la invitación de ${e.name}.`);
      }
    }
    const visit = (id, path) => {
      assert.ok(byId.has(id), `contact ${id}`);
      assert.ok(!path.includes(id), `cycle ${path} -> ${id}`);
      for (const p of byId.get(id).unlock.prerequisites) visit(p.engineer_id, [...path, id]);
    };
    visit(e.id, []);
  }
});
test('Odyssey referrals, mined goods and bonds are never represented as simple donations', () => {
  for (const e of entries.filter(e => e.on_foot || [300210, 300150, 300140, 300250, 300260, 300280].includes(e.id))) assert.equal(e.unlock.contribution, null, e.name);
  assert.deepEqual(entries.find(e => e.id === 400013).unlock.prerequisites.map(p => p.engineer_id).sort(), [400010, 400011, 400012]);
  assert.deepEqual(entries.find(e => e.id === 300010).unlock.prerequisites.map(p => p.engineer_id), [300210]);
});
