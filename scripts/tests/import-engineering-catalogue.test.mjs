import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, cpSync, readFileSync, writeFileSync, readdirSync, statSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { convertSnapshot, parseConstructors, validateAliases, sha256 } from '../import-engineering-catalogue.mjs';

const source = resolve('crates/tmc-modules/data/import/engineering/snapshot');
const staging = resolve(source, '../staging');
const importer = resolve('scripts/import-engineering-catalogue.mjs');
function temp(run) {
  const dir = mkdtempSync(join(tmpdir(), 'tmc-import-'));
  try { return run(dir); } finally { rmSync(dir, { recursive: true, force: true }); }
}
function changeSource(dir, filename, transform) {
  const file = join(dir, filename);
  writeFileSync(file, transform(readFileSync(file, 'utf8')));
  const manifest = JSON.parse(readFileSync(join(dir, 'sources.json')));
  manifest.files.find(f => f.file === filename).sha256 = sha256(readFileSync(file));
  writeFileSync(join(dir, 'sources.json'), JSON.stringify(manifest));
}

test('tokenizer preserves literals, comments, arrays, and constructor locations', () => {
  const rows = parseConstructors('// new EngineeringRecipe("ignored")\nnew EngineeringRecipe("a,\\\"b", "k", "1Fe", new [] { ItemData.ShipModule.ModuleTypes.MissileRack, ItemData.ShipModule.ModuleTypes.SeekerMissileRack }, 1, "Liz Ryder")', 'fixture.cs');
  assert.equal(rows.length, 1);
  assert.equal(rows[0].line, 2);
  assert.equal(rows[0].args[0], 'a,"b');
  assert.deepEqual(rows[0].args[3], [{ ref: 'ItemData.ShipModule.ModuleTypes.MissileRack' }, { ref: 'ItemData.ShipModule.ModuleTypes.SeekerMissileRack' }]);
  assert.equal(rows[0].original_sha256, sha256(rows[0].original_text));
});

test('pinned snapshot classifies every constructor and preserves every expansion and coin', () => {
  const result = convertSnapshot(source);
  const { rows, counts, coin_audit, engineer_id_audit } = result.manifest;
  assert.equal(rows.length, 1167);
  assert.equal(Object.values(counts).reduce((a, b) => a + b, 0), rows.length);
  assert.ok(rows.every(r => r.classification && r.reason && r.output_refs.length === r.modules.length));
  assert.equal(new Set(rows.map(r => r.row_id)).size, rows.length);
  assert.equal(coin_audit.input_total, coin_audit.output_total);
  assert.ok(coin_audit.rows > 105);
  assert.deepEqual(engineer_id_audit, { matched: 38, unique_ids: 38, missing: [], aliases: [] });
  assert.equal(result.engineers.entries.find(e => e.name === 'Felicity Farseer').id, 300100);
  assert.equal(result.engineers.entries.find(e => e.name === 'Elvira Martuuk').id, 300160);
  const missiles = rows.filter(r => r.key === 'weapon_highcapacity' && r.modules.includes('missilerack'));
  assert.equal(missiles.length, 5);
  for (const row of missiles) assert.deepEqual(row.modules, ['missilerack', 'seekermissilerack']);
});

test('source facts: dirty drives, Drag Drives, Operations coins, Lori capabilities', () => {
  const r = convertSnapshot(source);
  const dirty = r.blueprints.blueprints.find(b => b.key === 'engine_dirty').recipes.find(p => p.module_type === 'engine');
  assert.deepEqual(dirty.grades['1'], { materials: [{ material: 'legacyfirmware', count: 1 }], merc_coins: 0 });
  assert.deepEqual(dirty.grades['5'], { materials: [{ material: 'industrialfirmware', count: 1 }, { material: 'cadmium', count: 1 }, { material: 'pharmaceuticalisolators', count: 1 }], merc_coins: 0 });
  assert.deepEqual(r.blueprints.experimentals.find(e => e.key === 'special_engine_overloaded').cost.materials, [{ material: 'iron', count: 5 }, { material: 'hybridcapacitors', count: 3 }, { material: 'securityfirmware', count: 1 }]);
  const far = r.blueprints.blueprints.find(b => b.key === 'abrasionblaster_farreaching').recipes[0];
  assert.deepEqual(Object.values(far.grades).map(c => c.merc_coins), [5, 5, 10, 20]);
  assert.deepEqual(far.entry, { existing_only: { bases: [] } });
  const lori = r.engineers.entries.find(e => e.name === 'Lori Jameson');
  assert.equal(lori.blueprints.find(b => b.blueprint === 'misc_shielded' && b.module_type === 'autofieldmaintenanceunit').grades.at(-1), 4);
  assert.equal(lori.blueprints.find(b => b.blueprint === 'misc_shielded' && b.module_type === 'cargoscanner').grades.at(-1), 3);
});

test('changed SHA fails before parsing', () => temp(dir => {
  cpSync(source, dir, { recursive: true });
  writeFileSync(join(dir, 'eddiscovery-RecipesEngineering.cs'), 'changed');
  assert.throws(() => convertSnapshot(dir), /SHA-256 mismatch/);
}));
test('unknown constructor and unmapped ingredient fail with location', () => temp(dir => {
  cpSync(source, dir, { recursive: true });
  changeSource(dir, 'eddiscovery-RecipesEngineering.cs', s => s + '\nnew EngineeringRecipe("unrecognised");');
  assert.throws(() => convertSnapshot(dir), /RecipesEngineering.cs:\d+.*Unknown constructor/);
  cpSync(source, dir, { recursive: true });
  changeSource(dir, 'eddiscovery-RecipesEngineering.cs', s => s.replace('"1S,1HDP,1MCF"', '"1NEVERMAPPED"'));
  assert.throws(() => convertSnapshot(dir), /RecipesEngineering.cs:\d+.*Unmapped ingredient NEVERMAPPED/);
}));
test('ambiguous context aliases fail rather than taking first match', () => {
  assert.throws(() => validateAliases([{ module_type: 'engine', alias: 'a', canonical: 'x' }, { module_type: 'engine', alias: 'A', canonical: 'y' }]), /Ambiguous alias/);
});
test('conversion is deterministic and CLI --check is read-only and independent of cwd', () => temp(dir => {
  assert.deepEqual(convertSnapshot(source), convertSnapshot(source));
  execFileSync(process.execPath, [importer, '--source-dir', source, '--out-dir', dir], { cwd: tmpdir() });
  const before = Object.fromEntries(readdirSync(dir).map(f => [f, { sha: sha256(readFileSync(join(dir, f))), time: statSync(join(dir, f)).mtimeMs }]));
  execFileSync(process.execPath, [importer, '--source-dir', source, '--out-dir', dir, '--check'], { cwd: tmpdir() });
  assert.deepEqual(Object.fromEntries(readdirSync(dir).map(f => [f, { sha: sha256(readFileSync(join(dir, f))), time: statSync(join(dir, f)).mtimeMs }])), before);
  for (const f of readdirSync(dir)) assert.equal(readFileSync(join(dir, f), 'utf8'), readFileSync(join(staging, f), 'utf8'));
  writeFileSync(join(dir, 'blueprints.json'), '{}');
  assert.throws(() => execFileSync(process.execPath, [importer, '--source-dir', source, '--out-dir', dir, '--check'], { stdio: 'pipe' }), /Command failed/);
}));

test('final audit counts per category are pinned and unresolved ship rows stay explicit', () => {
  const { counts, summary, unresolved_ship_rows: unresolved } = convertSnapshot(source).manifest;
  assert.deepEqual(counts, { ship_recipe: 947, ship_experimental: 87, ship_variant: 15, out_of_scope: 116, obsolete: 2, unresolved: 0 });
  assert.equal(summary.constructors, 1167);
  assert.equal(summary.blueprints, 89);
  assert.equal(summary.recipes_by_blueprint_and_module, 207);
  assert.equal(summary.recipe_grade_rows, 987);
  assert.equal(summary.experimentals, 87);
  assert.equal(summary.variants, 15);
  assert.equal(summary.engineers, 38);
  assert.equal(unresolved.length, 0);
  assert.deepEqual(unresolved, []);
});

test('every ship recipe row keeps its own Merc Coins in the output and unresolved rows are not published', () => {
  const r = convertSnapshot(source);
  const published = new Map();
  for (const b of r.blueprints.blueprints) for (const p of b.recipes) for (const [g, cost] of Object.entries(p.grades)) published.set(`${b.key}/${p.module_type}/g${g}`, cost);
  for (const row of r.manifest.rows.filter(x => x.classification === 'ship_recipe')) {
    for (const ref of row.output_refs) {
      const cost = published.get(ref.replace('blueprints:', ''));
      assert.equal(cost.merc_coins, row.merc_coins, row.row_id);
      assert.deepEqual(cost.materials, row.ingredients.map(i => ({ material: i.material, count: i.count })), row.row_id);
    }
  }
  assert.ok(r.blueprints.blueprints.some(b => b.key === 'detailedsurfacescanner_longrange'));
  assert.ok(r.blueprints.experimentals.some(e => e.key === 'special_weapon_damage'));
});

test('plan oracle: dirty drives with rolls 1,1,1,1,4 total the hand-computed materials', () => {
  const r = convertSnapshot(source);
  const dirty = r.blueprints.blueprints.find(b => b.key === 'engine_dirty').recipes.find(p => p.module_type === 'engine');
  const total = {};
  const rolls = { 1: 1, 2: 1, 3: 1, 4: 1, 5: 4 };
  for (const [g, c] of Object.entries(dirty.grades)) for (const m of c.materials) total[m.material] = (total[m.material] ?? 0) + m.count * rolls[g];
  const exp = r.blueprints.experimentals.find(e => e.key === 'special_engine_overloaded').cost;
  for (const m of exp.materials) total[m.material] = (total[m.material] ?? 0) + m.count;
  assert.deepEqual(total, {
    legacyfirmware: 3, mechanicalequipment: 1, chromium: 1, mechanicalcomponents: 1, consumerfirmware: 1, selenium: 1,
    configurablecomponents: 1, industrialfirmware: 4, cadmium: 4, pharmaceuticalisolators: 4, iron: 5, hybridcapacitors: 3, securityfirmware: 1,
  });
});

test('cargo rack extended G5 keeps its coin and gets no invented engineer; placeholders are not engineers', () => {
  const r = convertSnapshot(source);
  const ext = r.blueprints.blueprints.find(b => b.key === 'cargoracks5c1_extended').recipes[0];
  assert.equal(ext.grades['5'].merc_coins, 1);
  assert.deepEqual(ext.entry, { existing_only: { bases: [] } });
  const rowsNoEng = r.manifest.ship_recipe_rows_without_engineer;
  assert.ok(rowsNoEng.length >= 2);
  const g5 = r.engineers.entries.flatMap(e => e.blueprints).filter(b => b.blueprint === 'cargoracks5c1_extended');
  assert.ok(g5.length > 0 && g5.every(b => !b.grades.includes(5)));
  assert.ok(!r.engineers.entries.some(e => e.name.toLowerCase() === 'tbd'));
});

test('pre-engineered rows become variants without a free recipe', () => {
  const r = convertSnapshot(source);
  const v = r.blueprints.variants.find(x => x.key === 'cargorack_increasedcapacity' && x.grade === 3);
  assert.equal(v.module_type, 'cargorack');
  assert.deepEqual(v.journal_blueprints, []);
  assert.ok(!r.blueprints.blueprints.some(b => b.key === 'cargorack_increasedcapacity'));
});

test('contradictory EDDI id/name pairs fail the exact 38/38 cross', () => temp(dir => {
  cpSync(source, dir, { recursive: true });
  changeSource(dir, 'eddi-Engineer.cs', s => s.replace('"Felicity Farseer"', '"Felicity Farseer Jr"'));
  assert.throws(() => convertSnapshot(dir), /Engineer ID cross failed/);
}));

test('unknown module type and unsupported syntax fail with file and line', () => temp(dir => {
  cpSync(source, dir, { recursive: true });
  changeSource(dir, 'eddiscovery-RecipesEngineering.cs', s => s + '\nnew EngineeringRecipe("x", "k", "1Fe", ItemData.ShipModule.ModuleTypes.NotAModule, 1, "");');
  assert.throws(() => convertSnapshot(dir), /RecipesEngineering.cs:\d+: Unknown module type NotAModule/);
  assert.throws(() => parseConstructors('new EngineeringRecipe("a" + "b")', 'f.cs'), /f\.cs:1: Unsupported argument syntax/);
  assert.throws(() => parseConstructors('/* open\nnew EngineeringRecipe("a")', 'f.cs'), /f\.cs:\d+: Unterminated block comment/);
}));

test('--check without --out-dir compares against the versioned staging and never writes', () => {
  const out = execFileSync(process.execPath, [importer, '--source-dir', source, '--check'], { cwd: tmpdir() }).toString();
  assert.match(out, /"checked"/);
  assert.throws(() => execFileSync(process.execPath, [importer, '--source-dir', source], { stdio: 'pipe' }), /Command failed/);
});
