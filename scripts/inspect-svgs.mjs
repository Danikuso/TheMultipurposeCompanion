import fs from 'node:fs';
import path from 'node:path';

const base = 'ui/src/assets/icons';

function checkSvg(p) {
  const full = path.join(base, p);
  const content = fs.readFileSync(full, 'utf8');
  const vb = content.match(/viewBox="([^"]+)"/);
  console.log(p.padEnd(35), 'viewBox:', vb ? vb[1] : 'none', 'len:', content.length);
}

const list = [
  'powers/aisling-duval.svg',
  'powers/zachary-hudson.svg',
  'powers/felicia-winters.svg',
  'powers/edmund-mahon.svg',
  'powers/li-yong-rui.svg',
  'superpowers/empire.svg',
  'superpowers/federation.svg',
  'superpowers/alliance.svg',
  'manufacturers/gutamaya.svg',
  'manufacturers/faulcon-delacy.svg',
  'manufacturers/core-dynamics.svg',
  'manufacturers/lakon.svg',
  'manufacturers/zorgon-peterson.svg',
  'manufacturers/saud-kruger.svg',
  'stations/mega-ship.svg',
  'stations/coriolis.svg',
  'stations/orbis.svg',
  'stations/ocellus.svg',
  'stations/outpost.svg',
  'engineering/engineer.svg',
  'engineering/grade-5.svg',
  'navigation/bookmark.svg',
  'navigation/ship.svg',
];

list.forEach(checkSvg);
