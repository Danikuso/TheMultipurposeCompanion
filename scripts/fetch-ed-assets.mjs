// scripts/fetch-ed-assets.mjs
// Fetches official / community vector assets from EDAssets (Venefilyn/EDAssets)
// for Powers, Superpowers, Ship Manufacturers, Stations, and Engineering.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

const BASE_RAW = 'https://raw.githubusercontent.com/Venefilyn/EDAssets/main/public/static/img';

const ASSETS_TO_FETCH = [
  // Powers (Powerplay)
  { category: 'powers', file: 'aisling-duval.svg', target: 'powers/aisling-duval.svg' },
  { category: 'powers', file: 'archon-delaine.svg', target: 'powers/archon-delaine.svg' },
  { category: 'powers', file: 'arissa-lavigny-duval.svg', target: 'powers/arissa-lavigny-duval.svg' },
  { category: 'powers', file: 'denton-patreus.svg', target: 'powers/denton-patreus.svg' },
  { category: 'powers', file: 'edmund-mahon.svg', target: 'powers/edmund-mahon.svg' },
  { category: 'powers', file: 'felicia-winters.svg', target: 'powers/felicia-winters.svg' },
  { category: 'powers', file: 'li-yong-rui.svg', target: 'powers/li-yong-rui.svg' },
  { category: 'powers', file: 'pranav-antal.svg', target: 'powers/pranav-antal.svg' },
  { category: 'powers', file: 'yuri-grom.svg', target: 'powers/yuri-grom.svg' },
  { category: 'powers', file: 'zachary-hudson.svg', target: 'powers/zachary-hudson.svg' },
  { category: 'powers', file: 'zemina-torval.svg', target: 'powers/zemina-torval.svg' },

  // Superpowers (Factions)
  { category: 'factions', file: 'Empire.svg', target: 'superpowers/empire.svg' },
  { category: 'factions', file: 'Federation.svg', target: 'superpowers/federation.svg' },
  { category: 'factions', file: 'Alliance.svg', target: 'superpowers/alliance.svg' },

  // Ship Manufacturers
  { category: 'manufacturers', file: 'logo-gutamaya.svg', target: 'manufacturers/gutamaya.svg' },
  { category: 'manufacturers', file: 'FaulconDelacy.svg', target: 'manufacturers/faulcon-delacy.svg' },
  { category: 'manufacturers', file: 'logo-core-dynamics.svg', target: 'manufacturers/core-dynamics.svg' },
  { category: 'manufacturers', file: 'lakon.svg', target: 'manufacturers/lakon.svg' },
  { category: 'manufacturers', file: 'logo-zorgon-peterson.svg', target: 'manufacturers/zorgon-peterson.svg' },
  { category: 'manufacturers', file: 'Saud_Kruger_bw.svg', target: 'manufacturers/saud-kruger.svg' },

  // Stations & Carriers
  { category: 'stations', file: 'Mega-Ship_Icon.svg', target: 'stations/mega-ship.svg' },
  { category: 'stations', file: 'Coriolis.svg', target: 'stations/coriolis.svg' },
  { category: 'stations', file: 'Orbis.svg', target: 'stations/orbis.svg' },
  { category: 'stations', file: 'Ocellus.svg', target: 'stations/ocellus.svg' },
  { category: 'stations', file: 'Outpost.svg', target: 'stations/outpost.svg' },

  // Engineering & Grades
  { category: 'engineers', file: 'Engineer_icon.svg', target: 'engineering/engineer.svg' },
  { category: 'materials', file: 'grade-1.svg', target: 'engineering/grade-1.svg' },
  { category: 'materials', file: 'grade-2.svg', target: 'engineering/grade-2.svg' },
  { category: 'materials', file: 'grade-3.svg', target: 'engineering/grade-3.svg' },
  { category: 'materials', file: 'grade-4.svg', target: 'engineering/grade-4.svg' },
  { category: 'materials', file: 'grade-5.svg', target: 'engineering/grade-5.svg' },
  { category: 'materials', file: 'material.svg', target: 'engineering/material.svg' },

  // Galaxy Map & Navigation Markers
  { category: 'galaxy-map/markers', file: 'bookmark.svg', target: 'navigation/bookmark.svg' },
  { category: 'galaxy-map/markers', file: 'ship.svg', target: 'navigation/ship.svg' },
  { category: 'galaxy-map/markers', file: 'communitygoal.svg', target: 'navigation/community-goal.svg' },

  // Portraits
  { category: 'portraits', file: 'Aisling-Duval-portrait.jpg', target: 'portraits/aisling-duval.jpg', isBinary: true },
  { category: 'portraits', file: 'Felicia-Winters-portrait.png', target: 'portraits/felicia-winters.png', isBinary: true },
  { category: 'portraits', file: 'Zachary-Hudson-portrait.png', target: 'portraits/zachary-hudson.png', isBinary: true },
];

const targetDir = path.resolve(rootDir, 'ui', 'src', 'assets', 'icons');

function normalizeSvg(content) {
  let svg = content;
  // Remove hardcoded black background styles so icon is transparent
  svg = svg.replace(/style="[^"]*background:\s*(black|#000|#000000)[^"]*"/gi, '');
  svg = svg.replace(/background:\s*(black|#000|#000000);?/gi, '');

  // Normalize CSS class fills: .cls-1{fill:#fff;} -> .cls-1{fill:currentColor;}
  svg = svg.replace(/fill:\s*(#fff|#ffffff|#000|#000000|black|white);?/gi, 'fill:currentColor;');

  // Normalize attribute fills: fill="#000000" or fill="#fff" -> fill="currentColor"
  svg = svg.replace(/fill="(#000000|#000|#ffffff|#fff|black|white)"/gi, 'fill="currentColor"');

  return svg;
}

async function downloadAsset(item) {
  const url = `${BASE_RAW}/${item.category}/${item.file}`;
  console.log(`Downloading: ${url} -> ${item.target}`);
  const res = await fetch(url, { headers: { 'User-Agent': 'TMC-Companion' } });
  if (!res.ok) {
    throw new Error(`Failed to download ${url}: ${res.status} ${res.statusText}`);
  }

  const destPath = path.resolve(targetDir, item.target);
  fs.mkdirSync(path.dirname(destPath), { recursive: true });

  if (item.isBinary) {
    const buffer = Buffer.from(await res.arrayBuffer());
    fs.writeFileSync(destPath, buffer);
  } else {
    let text = await res.text();
    const normalized = normalizeSvg(text);
    fs.writeFileSync(destPath, normalized, 'utf-8');
  }
}

async function main() {
  console.log(`Fetching EDAssets into ${targetDir}...`);
  for (const item of ASSETS_TO_FETCH) {
    try {
      await downloadAsset(item);
    } catch (e) {
      console.error(`Error downloading ${item.file}:`, e.message);
    }
  }
  console.log('Finished downloading assets.');
}

main();
