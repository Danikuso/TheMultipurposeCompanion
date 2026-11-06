// Etiquetas de rango: extracción Apache-2.0 de EDDI y complementos editoriales TMC.
// No descarga datos ni cambia los IDs numéricos del diario.
import { readFileSync, writeFileSync } from 'node:fs';

const snapshot = JSON.parse(readFileSync(new URL('./data/rank-localization.json', import.meta.url), 'utf8'));
const check = process.argv.includes('--check');
let changed = 0;
for (const lang of ['es', 'en', 'fr', 'de', 'pt']) {
  for (const file of ['main', 'combat', 'trade']) {
    const path = new URL(`../locales/${lang}/${file}.ftl`, import.meta.url);
    const original = readFileSync(path, 'utf8');
    const text = original.replace(/^(\w+)-rank-(\d+) = [^\r\n]*/gm, (line, family, index) => {
      const value = snapshot.editorialOverrides?.[lang]?.[family]?.[index] ?? snapshot.families[family]?.[lang]?.[Number(index)];
      if (!value) throw new Error(`Missing ${lang}:${family}:${index}`);
      return `${family}-rank-${index} = ${value}`;
    });
    if (text !== original) {
      changed++;
      if (!check) writeFileSync(path, text);
    }
  }
}
if (check && changed) throw new Error(`${changed} rank resources differ from the snapshot`);
console.log(`${check ? 'Checked' : 'Updated'} rank labels in five languages (${changed} files changed)`);
