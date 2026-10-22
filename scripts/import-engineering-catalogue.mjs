#!/usr/bin/env node
// Importador reproducible del catálogo de ingeniería de TMC (F4, unidad C1).
//
// Convierte un snapshot fijado (EDDiscovery Apache-2.0 + pares ID/nombre de EDDI Apache-2.0)
// en artefactos normalizados en staging. NO ejecuta ni integra código C#: tokeniza el subconjunto
// que necesita y falla con archivo:línea ante cualquier construcción que no comprende.
// NO descarga nada: la obtención de fuentes es un paso separado y explícito.
//
//   node scripts/import-engineering-catalogue.mjs --source-dir <snapshot> --out-dir <staging>
//   node scripts/import-engineering-catalogue.mjs --source-dir <snapshot> --check [--out-dir <staging>]
//
// Opcional (auditoría contra el catálogo activo, no afecta a la salida): --materials <materials.json>

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const FILES = {
  recipes: 'eddiscovery-RecipesEngineering.cs',
  engineers: 'eddiscovery-Engineers.cs',
  ingredients: 'eddiscovery-MCMRType.cs',
  modules: 'eddiscovery-Modules.cs',
  recipeFdName: 'eddiscovery-RecipeFDName.cs',
  eddi: 'eddi-Engineer.cs',
};
const MODULE_TYPES_PREFIX = 'ItemData.ShipModule.ModuleTypes.';
// IDs de módulo heredados del catálogo activo; el resto es el nombre de la enumeración en minúsculas.
const MODULE_ID_OVERRIDES = { Thrusters: 'engine', FrameShiftDrive: 'fsd' };
// Prefijos de ítem del Loadout con evidencia (catálogo activo F4). El resto queda sin símbolo comprobado.
const MODULE_ITEM_PREFIXES = { engine: ['int_engine_'], fsd: ['int_hyperdrive_'] };
// Marcadores de «ingeniero por determinar» de la fuente: no son ingenieros.
const PLACEHOLDER_ENGINEERS = new Set(['tbd']);
const SPECIAL_EFFECT_ENGINEER = 'Special Effect';
const CLASSES = ['ship_recipe', 'ship_experimental', 'ship_variant', 'out_of_scope', 'obsolete', 'unresolved'];

export function sha256(data) {
  return createHash('sha256').update(data).digest('hex');
}

class ImportError extends Error {}
function fail(file, line, message) {
  throw new ImportError(`${file}:${line}: ${message}`);
}

// ---------------------------------------------------------------------------------------------
// Tokenizador del subconjunto C#
// ---------------------------------------------------------------------------------------------

function lex(text, file) {
  const toks = [];
  const n = text.length;
  let i = 0;
  let line = 1;
  let lineStart = true;
  const push = (t, v, start, startLine) => toks.push({ t, v, pos: start, end: i, line: startLine });
  while (i < n) {
    const c = text[i];
    if (c === '\n') { line++; i++; lineStart = true; continue; }
    if (c === ' ' || c === '\t' || c === '\r' || c === '﻿') { i++; continue; }
    if (c === '#' && lineStart) { while (i < n && text[i] !== '\n') i++; continue; }
    lineStart = false;
    if (c === '/' && text[i + 1] === '/') { while (i < n && text[i] !== '\n') i++; continue; }
    if (c === '/' && text[i + 1] === '*') {
      i += 2;
      while (i < n && !(text[i] === '*' && text[i + 1] === '/')) { if (text[i] === '\n') line++; i++; }
      if (i >= n) fail(file, line, 'Unterminated block comment');
      i += 2;
      continue;
    }
    const start = i;
    const startLine = line;
    const verbatim = c === '@' && text[i + 1] === '"';
    if (c === '"' || verbatim || (c === '$' && text[i + 1] === '"')) {
      const interpolated = c === '$';
      i += c === '"' ? 1 : 2;
      let value = '';
      for (;;) {
        if (i >= n) fail(file, startLine, 'Unterminated string literal');
        const d = text[i];
        if (verbatim) {
          if (d === '"' && text[i + 1] === '"') { value += '"'; i += 2; continue; }
          if (d === '"') { i++; break; }
          if (d === '\n') line++;
          value += d; i++; continue;
        }
        if (d === '\\') {
          const e = text[i + 1];
          const map = { n: '\n', t: '\t', r: '\r', '0': '\0', '\\': '\\', '"': '"', "'": "'" };
          if (e in map) { value += map[e]; i += 2; continue; }
          if (e === 'u') { value += String.fromCharCode(parseInt(text.slice(i + 2, i + 6), 16)); i += 6; continue; }
          fail(file, line, `Unsupported escape sequence \\${e}`);
        }
        if (d === '"') { i++; break; }
        if (d === '\n') fail(file, line, 'Newline in string literal');
        value += d; i++;
      }
      push(interpolated ? 'istr' : 'str', value, start, startLine);
      continue;
    }
    if (c === "'") {
      i++;
      if (text[i] === '\\') i += 2; else i++;
      if (text[i] !== "'") fail(file, line, 'Unsupported character literal');
      i++;
      push('chr', text.slice(start, i), start, startLine);
      continue;
    }
    if (/[0-9]/.test(c)) {
      while (i < n && /[0-9A-Za-z_.]/.test(text[i])) i++;
      const raw = text.slice(start, i).replace(/[fFdDmMlLuU]+$/, '');
      push('num', Number(raw), start, startLine);
      continue;
    }
    if (/[A-Za-z_@]/.test(c)) {
      i++;
      while (i < n && /[A-Za-z0-9_]/.test(text[i])) i++;
      push('id', text.slice(start, i), start, startLine);
      continue;
    }
    i++;
    push('p', c, start, startLine);
  }
  return toks;
}

// Evaluador de argumentos: cadenas, enteros, booleanos, referencias con puntos y arrays `new [] {…}`.
function parseValue(cur) {
  const t = cur.toks[cur.i];
  if (!t) fail(cur.file, cur.lastLine, 'Unexpected end of file in argument list');
  if (t.t === 'str') { cur.i++; return t.v; }
  if (t.t === 'num') { cur.i++; return t.v; }
  if (t.t === 'p' && t.v === '-' && cur.toks[cur.i + 1]?.t === 'num') { cur.i += 2; return -cur.toks[cur.i - 1].v; }
  if (t.t === 'id' && (t.v === 'true' || t.v === 'false')) { cur.i++; return t.v === 'true'; }
  if (t.t === 'id' && t.v === 'new') {
    cur.i++;
    while (cur.toks[cur.i]?.t === 'id' || (cur.toks[cur.i]?.t === 'p' && cur.toks[cur.i].v === '.')) cur.i++;
    const open = cur.toks[cur.i];
    if (!(open?.v === '[' && cur.toks[cur.i + 1]?.v === ']' && cur.toks[cur.i + 2]?.v === '{')) {
      fail(cur.file, t.line, `Unsupported 'new' expression in argument`);
    }
    cur.i += 3;
    const items = [];
    for (;;) {
      if (cur.toks[cur.i]?.v === '}') { cur.i++; return items; }
      items.push(parseValue(cur));
      const sep = cur.toks[cur.i];
      if (sep?.v === ',') { cur.i++; continue; }
      if (sep?.v === '}') { cur.i++; return items; }
      fail(cur.file, sep?.line ?? t.line, 'Unsupported array initialiser syntax');
    }
  }
  if (t.t === 'id') {
    let ref = t.v;
    cur.i++;
    while (cur.toks[cur.i]?.v === '.' && cur.toks[cur.i + 1]?.t === 'id') { ref += `.${cur.toks[cur.i + 1].v}`; cur.i += 2; }
    return { ref };
  }
  return fail(cur.file, t.line, `Unsupported argument syntax near '${String(t.v)}'`);
}

function parseArgs(cur, openTok) {
  const args = [];
  cur.i++; // '('
  if (cur.toks[cur.i]?.v === ')') { cur.i++; return args; }
  for (;;) {
    args.push(parseValue(cur));
    const sep = cur.toks[cur.i];
    if (sep?.v === ',') { cur.i++; continue; }
    if (sep?.v === ')') { cur.i++; return args; }
    fail(cur.file, sep?.line ?? openTok.line, 'Unsupported argument syntax: expected "," or ")"');
  }
}

function makeCursor(text, file) {
  const toks = lex(text, file);
  return { toks, i: 0, file, lastLine: toks.at(-1)?.line ?? 1 };
}

// Todas las construcciones `new <className>(...)` fuera de comentarios y literales.
export function parseConstructors(text, file, className = 'EngineeringRecipe') {
  const cur = makeCursor(text, file);
  const rows = [];
  while (cur.i < cur.toks.length) {
    const t = cur.toks[cur.i];
    const name = cur.toks[cur.i + 1];
    const open = cur.toks[cur.i + 2];
    if (t.t === 'id' && t.v === 'new' && name?.t === 'id' && name.v === className && open?.v === '(') {
      cur.i += 2;
      const args = parseArgs(cur, t);
      const end = cur.toks[cur.i - 1].end;
      const original = text.slice(t.pos, end);
      rows.push({ file, line: t.line, ordinal: rows.length + 1, args, original_text: original, original_sha256: sha256(original) });
    } else cur.i++;
  }
  return rows;
}

// ---------------------------------------------------------------------------------------------
// Fuentes auxiliares
// ---------------------------------------------------------------------------------------------

function parseModuleTypes(text, file) {
  const cur = makeCursor(text, file);
  const names = [];
  for (let k = 0; k < cur.toks.length - 2; k++) {
    if (cur.toks[k].v === 'enum' && cur.toks[k + 1].v === 'ModuleTypes' && cur.toks[k + 2].v === '{') {
      for (let j = k + 3; cur.toks[j] && cur.toks[j].v !== '}'; j++) if (cur.toks[j].t === 'id') names.push(cur.toks[j].v);
      return names;
    }
  }
  return fail(file, 1, 'enum ModuleTypes not found');
}

// Add(CatType.X, [ItemType.X,] [MaterialGroupType.X,] MCMR.X, "english"[, "short"] ...) → abreviatura de ingrediente.
function parseIngredientTable(text, file) {
  const cur = makeCursor(text, file);
  const shorts = new Map();
  const collisions = new Map();
  let adds = 0;
  for (let k = 0; k < cur.toks.length - 3; k++) {
    const t = cur.toks[k];
    if (!(t.t === 'id' && t.v === 'Add' && cur.toks[k + 1].v === '(' && cur.toks[k + 2].v === 'CatType' && cur.toks[k - 1]?.v !== 'void')) continue;
    cur.i = k + 1;
    const args = parseArgs(cur, t);
    adds++;
    const refs = args.filter(a => a && typeof a === 'object' && !Array.isArray(a)).map(a => a.ref);
    const strs = args.filter(a => typeof a === 'string');
    const mcmr = refs.find(r => r.startsWith('MCMR.'));
    const cat = refs.find(r => r.startsWith('CatType.'));
    if (!mcmr || !cat) fail(file, t.line, 'Unsupported Add(…) overload');
    let fdname;
    let english;
    let short;
    if (strs.length === 1) { fdname = mcmr.slice(5); english = strs[0]; short = null; }
    else if (strs.length === 2) { fdname = mcmr.slice(5); [english, short] = strs; }
    else if (strs.length === 3) { [fdname, english, short] = strs; }
    else fail(file, t.line, 'Unsupported Add(…) overload');
    if (!short) continue;
    const entry = { short, material: fdname.toLowerCase(), name_en: english, category: cat.slice(8), line: t.line };
    if (shorts.has(short)) {
      const prev = shorts.get(short);
      if (prev.material !== entry.material) collisions.set(short, [...(collisions.get(short) ?? [prev]), entry]);
      continue;
    }
    shorts.set(short, entry);
  }
  return { shorts, collisions, adds };
}

function parseEngineersFile(text, file) {
  const cur = makeCursor(text, file);
  const list = [];
  for (let k = 2; k < cur.toks.length - 2; k++) {
    const t = cur.toks[k];
    if (!(t.t === 'id' && t.v === 'new' && cur.toks[k + 1].v === 'EngineeringInfo' && cur.toks[k + 2].v === '(')) continue;
    cur.i = k + 2;
    const a = parseArgs(cur, t);
    if (a.length !== 13 && a.length !== 11) fail(file, t.line, `Unknown constructor: EngineeringInfo with ${a.length} arguments`);
    const types = a.map(v => typeof v);
    const expected = ['string', 'string', 'string', 'number', 'number', 'number', 'string', 'string', 'string', 'string', 'string'];
    if (expected.some((e, j) => types[j] !== e)) fail(file, t.line, 'Unknown constructor: unexpected EngineeringInfo argument types');
    if (a.length === 13 && (typeof a[11] !== 'boolean' || typeof a[12] !== 'boolean')) fail(file, t.line, 'Unknown constructor: unexpected EngineeringInfo flags');
    const key = cur.toks[k - 2].t === 'str' ? cur.toks[k - 2].v : cur.toks[k - 1].t === 'str' ? cur.toks[k - 1].v : null;
    if (key !== a[0]) fail(file, t.line, `Dictionary key ${JSON.stringify(key)} differs from engineer name ${JSON.stringify(a[0])}`);
    list.push({
      line: t.line, name: a[0], system: a[1], workshop: a[2], star_pos: [a[3], a[4], a[5]], planet: a[6],
      discovery: a[7], meeting: a[8], unlock: a[9], reputation: a[10],
      permit_required: a.length === 13 ? a[11] : false, on_foot: a.length === 13 ? a[12] : false,
    });
  }
  return list;
}

// Solo (EngineerID, nombre): `new(300100, "Felicity Farseer", …)`.
function parseEddiPairs(text, file) {
  const cur = makeCursor(text, file);
  const pairs = [];
  for (let k = 0; k < cur.toks.length - 5; k++) {
    const t = cur.toks[k];
    if (t.t === 'id' && t.v === 'new' && cur.toks[k + 1].v === '(' && cur.toks[k + 2].t === 'num' && cur.toks[k + 3].v === ',' && cur.toks[k + 4].t === 'str') {
      pairs.push({ id: cur.toks[k + 2].v, name: cur.toks[k + 4].v, line: t.line });
    }
  }
  return pairs;
}

// ---------------------------------------------------------------------------------------------
// Aliases
// ---------------------------------------------------------------------------------------------

export function validateAliases(list) {
  const by = new Map();
  for (const a of list) {
    const key = `${a.module_type}\u0000${a.alias.toLowerCase()}`;
    const prev = by.get(key);
    if (prev && prev.canonical !== a.canonical) {
      throw new ImportError(`Ambiguous alias ${a.alias} in ${a.module_type}: ${prev.canonical} vs ${a.canonical}`);
    }
    if (!prev) by.set(key, { module_type: a.module_type, alias: a.alias.toLowerCase(), canonical: a.canonical });
  }
  return [...by.values()].sort(cmpBy('module_type', 'alias'));
}

function cmpBy(...fields) {
  return (a, b) => {
    for (const f of fields) {
      if (a[f] < b[f]) return -1;
      if (a[f] > b[f]) return 1;
    }
    return 0;
  };
}

// Reglas de `FindEngineeringFDName` convertidas en candidatos auditables (sin «primera coincidencia»).
function aliasCandidates(key, moduleSourceName, moduleId) {
  const underscore = key.lastIndexOf('_');
  if (underscore <= 0) return [];
  const first = key.slice(0, underscore);
  const last = key.slice(underscore);
  const mod = moduleSourceName.toLowerCase();
  let short = moduleSourceName;
  if (short === 'AutoFieldMaintenanceUnit') short = 'AFM';
  else if (short === 'CollectorLimpetController') short = 'CollectionLimpet';
  return [`${first}_${mod}${last}`, `${short.toLowerCase()}${last}`]
    .filter(a => a !== key)
    .map(alias => ({ module_type: moduleId, alias, canonical: key }));
}

// ---------------------------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------------------------

function readSnapshot(dir) {
  const manifestPath = join(dir, 'sources.json');
  if (!existsSync(manifestPath)) throw new ImportError(`${manifestPath}: sources.json not found`);
  const sources = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const texts = {};
  for (const f of sources.files) {
    const path = join(dir, f.file);
    if (!existsSync(path)) throw new ImportError(`${f.file}: missing from snapshot`);
    const buf = readFileSync(path);
    if (sha256(buf) !== f.sha256) throw new ImportError(`${f.file}: SHA-256 mismatch (expected ${f.sha256}, got ${sha256(buf)})`);
    const text = buf.toString('utf8');
    if (f.header_required) {
      const head = text.split('\n').slice(0, 20).join('\n');
      if (!/Apache License/.test(head)) throw new ImportError(`${f.file}: Apache-2.0 header not found`);
      if (!head.includes(f.copyright)) throw new ImportError(`${f.file}: copyright ${JSON.stringify(f.copyright)} not found in header`);
    } else if (f.license_file && !readFileSync(join(dir, f.license_file), 'utf8').includes(f.copyright_evidence ?? f.copyright)) {
      throw new ImportError(`${f.file}: copyright evidence not found in ${f.license_file}`);
    }
    texts[f.file] = text;
  }
  for (const name of Object.values(FILES)) if (!(name in texts)) throw new ImportError(`${name}: required file is not listed in sources.json`);
  return { sources, texts };
}

function parseIngredientList(list, shorts, collisions, file, line) {
  const out = [];
  if (list.trim() === '') return out;
  for (const part of list.split(',')) {
    const m = /^(\d+)(.+)$/.exec(part.trim());
    if (!m) fail(file, line, `Unparseable ingredient ${JSON.stringify(part)}`);
    const short = m[2];
    if (collisions.has(short)) fail(file, line, `Ambiguous ingredient ${short}`);
    const mapped = shorts.get(short);
    if (!mapped) fail(file, line, `Unmapped ingredient ${short}`);
    out.push({ short, material: mapped.material, count: Number(m[1]) });
  }
  return out;
}

function moduleId(sourceName) {
  return MODULE_ID_OVERRIDES[sourceName] ?? sourceName.toLowerCase();
}

const argType = a => (typeof a === 'string' ? 'S' : typeof a === 'number' ? 'I' : typeof a === 'boolean' ? 'B' : Array.isArray(a) ? 'A' : a && a.ref ? 'R' : '?');

const OVERLOADS = [
  { id: 'recipe', sig: /^SSS[RA]IS(I)?$/ },
  { id: 'broker', sig: /^SSSRS$/ },
  { id: 'effect_list', sig: /^SSSS$/ },
  { id: 'effect_single', sig: /^SSRS$/ },
  { id: 'suit_weapon_level', sig: /^RSIS$/ },
  { id: 'suit_weapon_mod', sig: /^RSSSISS$/ },
];

function shipModuleNames(enumNames) {
  const idx = enumNames.indexOf('DiscoveryScanner');
  return new Set(enumNames.slice(0, idx));
}

export function convertSnapshot(dir, options = {}) {
  const { sources, texts } = readSnapshot(dir);
  const enumNames = parseModuleTypes(texts[FILES.modules], FILES.modules);
  const enumSet = new Set(enumNames);
  const buyable = shipModuleNames(enumNames);
  const lowerEnum = new Map();
  for (const n of enumNames) {
    const k = n.toLowerCase();
    lowerEnum.set(k, lowerEnum.has(k) ? null : n);
  }
  const { shorts, collisions } = parseIngredientTable(texts[FILES.ingredients], FILES.ingredients);
  const engineerInfos = parseEngineersFile(texts[FILES.engineers], FILES.engineers);
  const eddiPairs = parseEddiPairs(texts[FILES.eddi], FILES.eddi);

  // --- Cruce de ingenieros: EDDiscovery (38) × EDDI (ID, nombre) ---------------------------
  const aliasMap = new Map((sources.engineer_aliases ?? []).map(a => [a.eddiscovery, a.eddi]));
  const eddiByName = new Map();
  for (const p of eddiPairs) {
    if (eddiByName.has(p.name)) throw new ImportError(`${FILES.eddi}:${p.line}: duplicate engineer name ${p.name}`);
    eddiByName.set(p.name, p);
  }
  const engineerNames = new Set();
  for (const e of engineerInfos) {
    if (engineerNames.has(e.name)) throw new ImportError(`${FILES.engineers}:${e.line}: duplicate engineer ${e.name}`);
    engineerNames.add(e.name);
  }
  const idByName = new Map();
  const usedAliases = [];
  const missing = [];
  for (const e of engineerInfos) {
    let p = eddiByName.get(e.name);
    if (!p && aliasMap.has(e.name)) { p = eddiByName.get(aliasMap.get(e.name)); usedAliases.push({ eddiscovery: e.name, eddi: aliasMap.get(e.name) }); }
    if (!p) { missing.push(e.name); continue; }
    idByName.set(e.name, p.id);
  }
  for (const p of eddiPairs) {
    const matched = engineerNames.has(p.name) || [...aliasMap.values()].includes(p.name);
    if (!matched) missing.push(`eddi:${p.name}`);
  }
  const uniqueIds = new Set(idByName.values()).size;
  const engineerIdAudit = { matched: idByName.size, unique_ids: uniqueIds, missing, aliases: usedAliases };
  if (missing.length || uniqueIds !== idByName.size || idByName.size !== engineerInfos.length || eddiPairs.length !== engineerInfos.length) {
    throw new ImportError(`Engineer ID cross failed: ${JSON.stringify({ ...engineerIdAudit, eddiscovery: engineerInfos.length, eddi: eddiPairs.length })}`);
  }
  for (const p of sources.preserved_engineer_ids ?? []) {
    if (idByName.get(p.name) !== p.id) throw new ImportError(`Engineer ID contradiction for ${p.name}: expected ${p.id}, got ${idByName.get(p.name)}`);
  }

  // --- Constructores ------------------------------------------------------------------------
  const rawRows = parseConstructors(texts[FILES.recipes], FILES.recipes);
  const rows = [];
  for (const raw of rawRows) {
    const sig = raw.args.map(argType).join('');
    const overload = OVERLOADS.find(o => o.sig.test(sig));
    if (!overload) fail(raw.file, raw.line, `Unknown constructor signature ${sig} (${raw.args.length} arguments)`);
    const row = {
      row_id: `${raw.file}:${raw.line}:${raw.ordinal}`,
      file: raw.file, line: raw.line, ordinal: raw.ordinal,
      original_text: raw.original_text, original_sha256: raw.original_sha256,
      constructor: overload.id,
      name: null, key: null, grade: null, merc_coins: 0,
      ingredients: [], engineers: [], source_modules: [], modules: [],
      classification: null, reason: null, output_refs: [],
    };
    const a = raw.args;
    const moduleRefs = refs => refs.map(r => {
      if (!r?.ref?.startsWith(MODULE_TYPES_PREFIX)) fail(raw.file, raw.line, `Unknown module type reference ${JSON.stringify(r)}`);
      const nm = r.ref.slice(MODULE_TYPES_PREFIX.length);
      if (!enumSet.has(nm)) fail(raw.file, raw.line, `Unknown module type ${nm}`);
      return nm;
    });
    const ingredients = list => parseIngredientList(list, shorts, collisions, raw.file, raw.line);
    const engineers = list => {
      const names = list.trim() === '' ? [] : list.split(',').map(s => s.trim());
      for (const nm of names) {
        if (PLACEHOLDER_ENGINEERS.has(nm) || nm === SPECIAL_EFFECT_ENGINEER) continue;
        if (!engineerNames.has(nm)) fail(raw.file, raw.line, `Unknown engineer ${nm}`);
      }
      return names;
    };
    switch (overload.id) {
      case 'recipe':
        row.name = a[0]; row.key = a[1].toLowerCase(); row.ingredients = ingredients(a[2]);
        row.source_modules = moduleRefs(Array.isArray(a[3]) ? a[3] : [a[3]]);
        row.grade = a[4]; row.engineers = engineers(a[5]); row.merc_coins = a[6] ?? 0;
        break;
      case 'broker':
        row.name = a[0]; row.key = a[1].toLowerCase(); row.source_modules = moduleRefs([a[3]]);
        row.ingredients = ingredients(a[4]); row.broker_type = a[2];
        break;
      case 'effect_list':
        row.name = a[0]; row.key = a[1].toLowerCase(); row.ingredients = ingredients(a[3]); row.engineers = [SPECIAL_EFFECT_ENGINEER];
        row.source_modules = a[2].split(',').map(s => {
          const nm = lowerEnum.get(s.trim().toLowerCase());
          if (!nm) fail(raw.file, raw.line, nm === null ? `Ambiguous module type ${s}` : `Unknown module type ${s}`);
          return nm;
        });
        break;
      case 'effect_single':
        row.name = a[0]; row.key = a[1].toLowerCase(); row.source_modules = moduleRefs([a[2]]);
        row.ingredients = ingredients(a[3]); row.engineers = [SPECIAL_EFFECT_ENGINEER];
        break;
      case 'suit_weapon_level':
        row.name = a[1]; row.source_modules = moduleRefs([a[0]]); row.grade = a[2]; row.ingredients = ingredients(a[3]);
        break;
      case 'suit_weapon_mod':
        row.name = a[3]; row.key = a[1].toLowerCase(); row.source_modules = moduleRefs([a[0]]);
        row.ingredients = ingredients(a[5]); row.engineers = engineers(a[6]); row.cost_credits = a[4]; row.variant = a[2];
        break;
      default: break;
    }
    rows.push(row);
  }
  if (new Set(rows.map(r => r.row_id)).size !== rows.length) throw new ImportError('row_id collision');

  // --- Clasificación ------------------------------------------------------------------------
  // Tipos genéricos que sí identifican un módulo de nave; el resto de no comprables (Weapon, Fighter,
  // Suit, DiscoveryScanner, UnknownType…) no se interpretan como módulo concreto de nave.
  const SHIP_GENERIC = new Set(['Armour']);
  const classify = (row, cls, reason) => { row.classification = cls; row.reason = reason; };
  for (const row of rows) {
    const mods = row.source_modules;
    const hasUnknown = mods.includes('UnknownType');
    const nonShip = mods.filter(m => !buyable.has(m) && !SHIP_GENERIC.has(m));
    switch (row.constructor) {
      case 'suit_weapon_level':
      case 'suit_weapon_mod':
        classify(row, 'out_of_scope', 'Mejora de traje/arma de Odyssey (a pie): fuera del alcance de F4 (enmienda §3.2)');
        break;
      case 'broker':
        classify(row, 'out_of_scope', 'Adquisición en el Tech Broker (compra de módulo con materiales): no es blueprint ni experimental de ingeniería de nave (enmienda §3.2)');
        break;
      case 'effect_list':
      case 'effect_single':
        if (hasUnknown || nonShip.length) classify(row, 'unresolved', `Efecto especial con tipo de módulo no interpretable: ${nonShip.join(',')}`);
        else if (row.ingredients.length === 0) classify(row, 'unresolved', 'Efecto especial sin ingredientes');
        else classify(row, 'ship_experimental', 'Efecto especial (experimental) de módulos de nave');
        break;
      case 'recipe':
        if (hasUnknown) {
          if (/obsolete/i.test(row.name)) classify(row, 'obsolete', 'Fila obsoleta de la fuente (UnknownType, nombre «Obsolete», comentario «wrong data»)');
          else classify(row, 'unresolved', 'Módulo UnknownType sin marca de obsoleto');
        } else if (nonShip.length) classify(row, 'unresolved', `Tipo de módulo no interpretable: ${nonShip.join(',')}`);
        else if (!Number.isInteger(row.grade) || row.grade < 1 || row.grade > 5) classify(row, 'unresolved', `Grado fuera de 1..5: ${row.grade}`);
        else if (row.ingredients.length > 0 || row.merc_coins > 0) classify(row, 'ship_recipe', 'Receta de ingeniería de nave por módulo y grado');
        else classify(row, 'ship_variant', 'Variante preingenierizada: sin ingredientes ni Merc Coins (indicio PreEngineered, no receta gratuita)');
        break;
      default: classify(row, 'unresolved', 'Sobrecarga sin clasificación');
    }
    row.modules = ['ship_recipe', 'ship_experimental', 'ship_variant'].includes(row.classification) ? [...new Set(mods.map(moduleId))] : [];
  }

  // --- Salidas por clase --------------------------------------------------------------------
  const recipeKey = (key, mod) => `${key}\u0000${mod}`;
  const blueprintMap = new Map(); // key → { names, contexts: Map(mod → {grades: Map(grade → {cost, rows, engineers})}) }
  const experimentalMap = new Map(); // key → { names, modules:Set, cost, rows }
  const variantMap = new Map(); // key|mod|grade → entry
  const conflicts = [];
  const costOf = row => ({
    materials: row.ingredients.map(i => ({ material: i.material, count: i.count })),
    merc_coins: row.merc_coins,
  });
  const costSig = c => JSON.stringify(c);

  for (const row of rows) {
    if (row.classification === 'ship_recipe') {
      let bp = blueprintMap.get(row.key);
      if (!bp) { bp = { names: new Set(), contexts: new Map() }; blueprintMap.set(row.key, bp); }
      bp.names.add(row.name);
      for (const mod of row.modules) {
        let ctx = bp.contexts.get(mod);
        if (!ctx) { ctx = new Map(); bp.contexts.set(mod, ctx); }
        const cost = costOf(row);
        const prev = ctx.get(row.grade);
        const eng = [...row.engineers].sort().join(',');
        if (prev && (costSig(prev.cost) !== costSig(cost) || prev.eng !== eng)) {
          conflicts.push({ key: row.key, module_type: mod, grade: row.grade, rows: [...prev.rows, row.row_id] });
          prev.conflict = true; prev.rows.push(row.row_id);
        } else if (prev) prev.rows.push(row.row_id);
        else ctx.set(row.grade, { cost, eng, rows: [row.row_id] });
      }
    } else if (row.classification === 'ship_experimental') {
      let ex = experimentalMap.get(row.key);
      const cost = { materials: row.ingredients.map(i => ({ material: i.material, count: i.count })), merc_coins: 0 };
      if (!ex) { ex = { names: new Set(), modules: new Set(), cost, rows: [] }; experimentalMap.set(row.key, ex); }
      else if (costSig(ex.cost) !== costSig(cost)) { conflicts.push({ key: row.key, experimental: true, rows: [...ex.rows, row.row_id] }); ex.conflict = true; }
      ex.names.add(row.name); ex.rows.push(row.row_id);
      for (const m of row.modules) ex.modules.add(m);
    } else if (row.classification === 'ship_variant') {
      for (const mod of row.modules) {
        const id = `${row.key}\u0000${mod}\u0000${row.grade}`;
        const v = variantMap.get(id) ?? { key: row.key, module_type: mod, grade: row.grade, names: new Set(), rows: [] };
        v.names.add(row.name); v.rows.push(row.row_id);
        variantMap.set(id, v);
      }
    }
  }
  // Conflictos: ninguna fila en conflicto se publica; pasan a `unresolved` (explícitas, bloquean completitud).
  const conflictRows = new Set(conflicts.flatMap(c => c.rows));
  for (const row of rows) {
    if (conflictRows.has(row.row_id)) classify(row, 'unresolved', 'Duplicado con costes/ingenieros distintos para el mismo (blueprint, módulo, grado): sin fusión posible');
  }
  if (conflicts.length) {
    for (const [key, bp] of blueprintMap) for (const [mod, ctx] of bp.contexts) for (const [g, e] of ctx) if (e.conflict) ctx.delete(g);
    for (const [key, ex] of experimentalMap) if (ex.conflict) experimentalMap.delete(key);
    for (const row of rows) if (row.classification === 'unresolved') { row.modules = []; }
  }

  // --- blueprints.json ----------------------------------------------------------------------
  const usedModuleSources = new Map(); // id → source enum name
  for (const row of rows) row.source_modules.forEach(m => { if (row.modules.length) usedModuleSources.set(moduleId(m), m); });
  const sourceTag = `eddiscovery@${sources.eddiscovery_commit.slice(0, 7)}`;
  const blueprints = [];
  const bpOut = new Map();
  for (const key of [...blueprintMap.keys()].sort()) {
    const bp = blueprintMap.get(key);
    const recipes = [];
    for (const mod of [...bp.contexts.keys()].sort()) {
      const ctx = bp.contexts.get(mod);
      if (ctx.size === 0) continue;
      const gradeList = [...ctx.keys()].sort((x, y) => x - y);
      const grades = {};
      for (const g of gradeList) grades[g] = ctx.get(g).cost;
      const gaps = [];
      for (let g = gradeList[0]; g <= gradeList.at(-1); g++) if (!ctx.has(g)) gaps.push(g);
      recipes.push({
        module_type: mod, selector: null,
        entry: gradeList[0] === 1 ? 'stock' : { existing_only: { bases: [] } },
        grades, ...(gaps.length ? { grade_gaps: gaps } : {}),
      });
      bpOut.set(recipeKey(key, mod), { grades: ctx });
    }
    if (!recipes.length) continue;
    blueprints.push({ key, names_en: [...bp.names].sort(), recipes, source: `${sourceTag}:RecipesEngineering.cs (Apache-2.0)`, verified: false });
  }
  const experimentals = [];
  for (const key of [...experimentalMap.keys()].sort()) {
    const ex = experimentalMap.get(key);
    experimentals.push({ key, names_en: [...ex.names].sort(), module_types: [...ex.modules].sort(), cost: ex.cost, source: `${sourceTag}:RecipesEngineering.cs (Apache-2.0)`, verified: false });
  }
  const variants = [...variantMap.values()]
    .sort(cmpBy('key', 'module_type', 'grade'))
    .map(v => ({ key: v.key, names_en: [...v.names].sort(), module_type: v.module_type, grade: v.grade, selector: null, journal_blueprints: [], source: `${sourceTag}:RecipesEngineering.cs (Apache-2.0)`, verified: false }));
  const moduleTypes = [...usedModuleSources.entries()].sort(([a], [b]) => (a < b ? -1 : 1)).map(([id, src]) => ({
    id, source_name: src, item_prefixes: MODULE_ITEM_PREFIXES[id] ?? [], symbol_status: MODULE_ITEM_PREFIXES[id] ? 'verified_active_catalogue' : 'pending_evidence',
  }));
  const provenance = provenanceBlock(sources);

  // --- Referencias de salida por fila -------------------------------------------------------
  for (const row of rows) {
    if (row.classification === 'ship_recipe') row.output_refs = row.modules.map(m => `blueprints:${row.key}/${m}/g${row.grade}`);
    else if (row.classification === 'ship_experimental') row.output_refs = row.modules.map(m => `experimentals:${row.key}/${m}`);
    else if (row.classification === 'ship_variant') row.output_refs = row.modules.map(m => `variants:${row.key}/${m}/g${row.grade}`);
  }
  const outputCoin = ref => {
    const m = /^blueprints:(.+)\/([^/]+)\/g(\d)$/.exec(ref);
    if (!m) return 0;
    return bpOut.get(recipeKey(m[1], m[2]))?.grades.get(Number(m[3]))?.cost.merc_coins ?? 0;
  };
  let inputTotal = 0;
  let outputTotal = 0;
  let coinRows = 0;
  let coinRowsOutside = 0;
  let coinRowsUnresolved = 0;
  let coinUnresolvedTotal = 0;
  const coinOutsideRows = [];
  for (const row of rows) {
    if (row.merc_coins > 0) {
      if (row.classification === 'ship_recipe') { coinRows++; inputTotal += row.merc_coins * row.modules.length; }
      else if (row.classification === 'unresolved') { coinRowsUnresolved++; coinUnresolvedTotal += row.merc_coins; }
      else { coinRowsOutside++; coinOutsideRows.push(`${row.row_id} ${row.classification}`); }
    }
    if (row.classification === 'ship_recipe') for (const ref of row.output_refs) outputTotal += outputCoin(ref);
  }
  const coinAudit = { rows: coinRows, input_total: inputTotal, output_total: outputTotal, rows_with_coins_unresolved: coinRowsUnresolved, coins_unresolved_total: coinUnresolvedTotal, rows_with_coins_in_other_classes: coinRowsOutside };
  if (inputTotal !== outputTotal || coinRowsOutside) throw new ImportError(`Merc Coins lost or unclassified: ${JSON.stringify(coinAudit)} ${coinOutsideRows.join('; ')}`);

  // --- Ingenieros ---------------------------------------------------------------------------
  const capMap = new Map(); // name → Map(mod|key → Set(grade))
  const rowsWithoutEngineer = [];
  for (const row of rows) {
    if (row.classification !== 'ship_recipe') continue;
    const real = row.engineers.filter(n => engineerNames.has(n));
    if (!real.length) { rowsWithoutEngineer.push(row.row_id); continue; }
    for (const name of real) {
      const m = capMap.get(name) ?? new Map();
      capMap.set(name, m);
      for (const mod of row.modules) {
        const k = `${mod}\u0000${row.key}`;
        const set = m.get(k) ?? new Set();
        set.add(row.grade);
        m.set(k, set);
      }
    }
  }
  const slug = name => name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  const engineers = engineerInfos.map(e => {
    const caps = [...(capMap.get(e.name)?.entries() ?? [])].map(([k, set]) => {
      const [module_type, blueprint] = k.split('\u0000');
      return { module_type, blueprint, grades: [...set].sort((x, y) => x - y) };
    }).sort(cmpBy('module_type', 'blueprint'));
    return {
      id: idByName.get(e.name), name: e.name, system: e.system, star_pos: e.star_pos, workshop: e.workshop, planet: e.planet,
      market_id: null, on_foot: e.on_foot, permit_required: e.permit_required,
      blueprints: caps,
      unlock: { requirement_key: `unlock-${slug(e.name)}`, invite_key: `unlock-invite-${slug(e.name)}`, contribution: null },
      source_text: { discovery: e.discovery, meeting: e.meeting, unlock: e.unlock, reputation: e.reputation },
      source: `${sourceTag}:Engineers.cs + eddi@${sources.eddi_commit.slice(0, 7)}:Engineer.cs ids (Apache-2.0)`, verified: false,
    };
  }).sort((x, y) => x.id - y.id);

  // --- Aliases ------------------------------------------------------------------------------
  const cand = new Map();
  for (const row of rows) {
    if (row.classification !== 'ship_recipe' && row.classification !== 'ship_variant') continue;
    row.source_modules.forEach((m, idx) => {
      for (const a of aliasCandidates(row.key, m, moduleId(m))) {
        const k = `${a.module_type}\u0000${a.alias}`;
        const set = cand.get(k) ?? { module_type: a.module_type, alias: a.alias, canon: new Set() };
        set.canon.add(a.canonical);
        cand.set(k, set);
      }
    });
  }
  const aliasList = [];
  const ambiguousAliases = [];
  for (const c of [...cand.values()].sort(cmpBy('module_type', 'alias'))) {
    if (c.canon.size === 1) aliasList.push({ module_type: c.module_type, alias: c.alias, canonical: [...c.canon][0] });
    else ambiguousAliases.push({ module_type: c.module_type, alias: c.alias, candidates: [...c.canon].sort() });
  }
  for (const a of sources.explicit_aliases ?? []) aliasList.push(a);
  const aliases = validateAliases(aliasList);

  // --- Símbolos / ingredientes usados -------------------------------------------------------
  const usedShorts = new Set(rows.flatMap(r => r.ingredients.map(i => i.short)));
  const ingredientTable = [...usedShorts].sort().map(s => {
    const e = shorts.get(s);
    return { short: s, material: e.material, name_en: e.name_en, category: e.category };
  });
  const symbols = { provenance, module_types: moduleTypes, ingredients: ingredientTable, aliases, ambiguous_aliases_excluded: ambiguousAliases };

  // --- Manifiesto ---------------------------------------------------------------------------
  const counts = Object.fromEntries(CLASSES.map(c => [c, rows.filter(r => r.classification === c).length]));
  const unresolved = rows.filter(r => r.classification === 'unresolved').map(r => ({ row_id: r.row_id, key: r.key, name: r.name, modules: r.source_modules, grade: r.grade, reason: r.reason }));
  const byCtor = {};
  for (const r of rows) byCtor[r.constructor] = (byCtor[r.constructor] ?? 0) + 1;
  const perClassByCtor = {};
  for (const r of rows) { const k = `${r.constructor}/${r.classification}`; perClassByCtor[k] = (perClassByCtor[k] ?? 0) + 1; }
  const recipeContexts = blueprints.reduce((n, b) => n + b.recipes.length, 0);
  const gradeRows = blueprints.reduce((n, b) => n + b.recipes.reduce((m, r) => m + Object.keys(r.grades).length, 0), 0);
  const summary = {
    constructors: rows.length,
    constructors_by_overload: byCtor,
    constructors_by_overload_and_class: perClassByCtor,
    blueprints: blueprints.length,
    recipes_by_blueprint_and_module: recipeContexts,
    recipe_grade_rows: gradeRows,
    experimentals: experimentals.length,
    experimental_module_expansions: experimentals.reduce((n, e) => n + e.module_types.length, 0),
    variants: variants.length,
    module_types: moduleTypes.length,
    engineers: engineers.length,
    engineer_capabilities: engineers.reduce((n, e) => n + e.blueprints.length, 0),
    ingredient_symbols_used: ingredientTable.length,
    aliases: aliases.length,
    ambiguous_aliases_excluded: ambiguousAliases.length,
    stock_recipes: blueprints.reduce((n, b) => n + b.recipes.filter(r => r.entry === 'stock').length, 0),
    existing_only_recipes: blueprints.reduce((n, b) => n + b.recipes.filter(r => r.entry !== 'stock').length, 0),
    recipes_with_grade_gaps: blueprints.reduce((n, b) => n + b.recipes.filter(r => r.grade_gaps).length, 0),
  };
  const gradeGaps = blueprints.flatMap(b => b.recipes.filter(r => r.grade_gaps).map(r => ({ key: b.key, module_type: r.module_type, missing_grades: r.grade_gaps })));
  const manifest = {
    provenance,
    files: sources.files.map(f => ({ ...f })),
    counts,
    summary,
    coin_audit: coinAudit,
    engineer_id_audit: engineerIdAudit,
    unresolved_ship_rows: unresolved,
    ship_recipe_rows_without_engineer: rowsWithoutEngineer,
    grade_gaps: gradeGaps,
    rows,
  };
  if (options.materialsFile) manifest.materials_audit = materialsAudit(options.materialsFile, rows);
  const outputs = { blueprints: { provenance, module_types: moduleTypes, blueprints, experimentals, variants }, engineers: { provenance, engineers }, symbols, manifest };
  return outputs;
}

function provenanceBlock(sources) {
  return {
    notice: 'Datos derivados de EDDiscovery/EliteDangerousCore (Apache-2.0) y, solo para pares ID/nombre de ingenieros, de EDCD/EDDI (Apache-2.0). Convertidos y modificados por TMC con scripts/import-engineering-catalogue.mjs; entradas sin verificar contra el juego (verified:false).',
    eddiscovery: { repository: sources.eddiscovery_repository, commit: sources.eddiscovery_commit },
    eddi: { repository: sources.eddi_repository, commit: sources.eddi_commit, use: 'solo pares (EngineerID, nombre)' },
  };
}

function materialsAudit(path, rows) {
  const known = new Set(JSON.parse(readFileSync(path, 'utf8')).entries.map(e => e.key));
  const used = new Set();
  for (const r of rows) if (['ship_recipe', 'ship_experimental'].includes(r.classification)) r.ingredients.forEach(i => used.add(i.material));
  return { materials_used: used.size, missing_in_materials_json: [...used].filter(m => !known.has(m)).sort() };
}

// ---------------------------------------------------------------------------------------------
// Salida y CLI
// ---------------------------------------------------------------------------------------------

export function renderOutputs(result) {
  const json = v => `${JSON.stringify(v, null, 2)}\n`;
  return {
    'blueprints.json': json(result.blueprints),
    'engineers.json': json(result.engineers),
    'symbols.json': json(result.symbols),
    'engineering-catalogue-manifest.json': json(result.manifest),
  };
}

function parseCli(argv) {
  const opts = { check: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--check') opts.check = true;
    else if (a === '--source-dir') opts.sourceDir = argv[++i];
    else if (a === '--out-dir') opts.outDir = argv[++i];
    else if (a === '--materials') opts.materials = argv[++i];
    else throw new ImportError(`Unknown argument ${a}`);
  }
  if (!opts.sourceDir) throw new ImportError('Usage: --source-dir <snapshot> (--out-dir <staging> | --check [--out-dir <staging>])');
  if (!opts.check && !opts.outDir) throw new ImportError('--out-dir is required unless --check is given');
  return opts;
}

export function run(argv) {
  const opts = parseCli(argv);
  const sourceDir = resolve(opts.sourceDir);
  const result = convertSnapshot(sourceDir, { materialsFile: opts.materials ? resolve(opts.materials) : undefined });
  const outputs = renderOutputs(result);
  const outDir = resolve(opts.outDir ?? join(sourceDir, '..', 'staging'));
  if (opts.check) {
    const problems = [];
    const present = existsSync(outDir) ? readdirSync(outDir) : [];
    for (const [name, content] of Object.entries(outputs)) {
      const path = join(outDir, name);
      if (!existsSync(path)) problems.push(`${name}: missing`);
      else if (readFileSync(path, 'utf8') !== content) problems.push(`${name}: differs from regenerated output`);
    }
    for (const f of present) if (!(f in outputs)) problems.push(`${f}: unexpected file`);
    if (problems.length) throw new ImportError(`--check failed:\n${problems.join('\n')}`);
    return { checked: Object.keys(outputs), counts: result.manifest.counts };
  }
  mkdirSync(outDir, { recursive: true });
  for (const [name, content] of Object.entries(outputs)) writeFileSync(join(outDir, name), content);
  return { written: Object.keys(outputs), counts: result.manifest.counts };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const r = run(process.argv.slice(2));
    console.log(JSON.stringify(r));
  } catch (e) {
    console.error(e instanceof ImportError ? e.message : e);
    process.exit(1);
  }
}

