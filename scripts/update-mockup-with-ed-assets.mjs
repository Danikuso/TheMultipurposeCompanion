// scripts/update-mockup-with-ed-assets.mjs
// Embeds official Elite Dangerous vector icons and portraits into the F5 interactive mockup

import fs from 'node:fs';
import path from 'node:path';

const mockupPath = 'C:/Users/danik/.gemini/antigravity/brain/d535e4e5-ea82-44a5-82ea-da3439e3b468/f5_carrier_colonisation_powerplay_mockups.html';
const iconsBase = 'ui/src/assets/icons';

function readSvg(relPath) {
  const full = path.join(iconsBase, relPath);
  return fs.readFileSync(full, 'utf8').trim();
}

const empireSvg = readSvg('superpowers/empire.svg');
const federationSvg = readSvg('superpowers/federation.svg');
const allianceSvg = readSvg('superpowers/alliance.svg');
const gutamayaSvg = readSvg('manufacturers/gutamaya.svg');
const megashipSvg = readSvg('stations/mega-ship.svg');
const outpostSvg = readSvg('stations/outpost.svg');
const aislingSvg = readSvg('powers/aisling-duval.svg');
const coriolisSvg = readSvg('stations/coriolis.svg');

let html = fs.readFileSync(mockupPath, 'utf8');

// 1. Add "ICONOS OFICIALES ED" button in header
const headerStatusTarget = `<div class="flex items-center gap-3">
      <div class="text-right">
        <div class="text-xs text-[var(--hud-muted)]">ESTADO COMPANION</div>
        <div class="text-sm font-semibold text-[var(--hud-ok)] flex items-center gap-1.5 justify-end">
          <span class="w-2 h-2 rounded-full bg-[var(--hud-ok)] animate-pulse"></span>
          ENLACE ACTIVO · 10 HZ
        </div>
      </div>
    </div>`;

const newHeaderStatus = `<div class="flex items-center gap-4">
      <button class="hud-btn text-xs py-1.5 px-3.5 border border-[var(--hud-cyan)] text-[var(--hud-cyan)] bg-[rgba(0,217,255,0.08)] hover:bg-[rgba(0,217,255,0.22)] shadow-[0_0_15px_rgba(0,217,255,0.25)] flex items-center gap-2" onclick="openAssetCatalogModal()">
        <span class="w-4 h-4 inline-block text-[var(--hud-cyan)]">${empireSvg.replace('<svg ', '<svg class="w-full h-full" ')}</span>
        <span class="font-bold tracking-wider">BIBLIOTECA DE ICONOS OFICIALES</span>
      </button>
      <div class="text-right">
        <div class="text-xs text-[var(--hud-muted)]">ESTADO COMPANION</div>
        <div class="text-sm font-semibold text-[var(--hud-ok)] flex items-center gap-1.5 justify-end">
          <span class="w-2 h-2 rounded-full bg-[var(--hud-ok)] animate-pulse"></span>
          ENLACE ACTIVO · 10 HZ
        </div>
      </div>
    </div>`;

if (html.includes(headerStatusTarget)) {
  html = html.replace(headerStatusTarget, newHeaderStatus);
  console.log('Added "BIBLIOTECA DE ICONOS OFICIALES" button in mockup header!');
}

// 2. Replace emojis in .nav-tab with clean vectors
const navTabsTarget = `  <!-- Pestañas de Navegación de la Maqueta -->
  <div class="max-w-7xl mx-auto mb-6 flex border-b border-[var(--hud-border)] overflow-x-auto">
    <button class="nav-tab active" onclick="switchTab('carrier')">
      🛸 1. Fleet Carrier
    </button>
    <button class="nav-tab" onclick="switchTab('colonisation')">
      🏗️ 2. Colonización (Trailblazers)
    </button>
    <button class="nav-tab" onclick="switchTab('powerplay')">
      ⚡ 3. Powerplay 2.0 (Ascendancy)
    </button>
    <button class="nav-tab" onclick="switchTab('bgs')">
      🪐 4. BGS & Galaxia Viva
    </button>
    <button class="nav-tab" onclick="switchTab('onfoot')">
      🥾 5. A Pie (Odyssey)
    </button>
  </div>`;

const newNavTabs = `  <!-- Pestañas de Navegación de la Maqueta -->
  <div class="max-w-7xl mx-auto mb-6 flex border-b border-[var(--hud-border)] overflow-x-auto">
    <button class="nav-tab active flex items-center gap-2" onclick="switchTab('carrier')">
      <span class="w-4 h-4 inline-block text-[var(--hud-primary)]">${megashipSvg.replace('<svg ', '<svg class="w-full h-full" ')}</span>
      <span>1. Fleet Carrier</span>
    </button>
    <button class="nav-tab flex items-center gap-2" onclick="switchTab('colonisation')">
      <span class="w-4 h-4 inline-block text-[var(--hud-primary)]">${outpostSvg.replace('<svg ', '<svg class="w-full h-full" ')}</span>
      <span>2. Colonización (Trailblazers)</span>
    </button>
    <button class="nav-tab flex items-center gap-2" onclick="switchTab('powerplay')">
      <span class="w-4 h-4 inline-block text-[var(--hud-cyan)]">${aislingSvg.replace('<svg ', '<svg class="w-full h-full" ')}</span>
      <span>3. Powerplay 2.0 (Ascendancy)</span>
    </button>
    <button class="nav-tab flex items-center gap-2" onclick="switchTab('bgs')">
      <span class="w-4 h-4 inline-block text-[var(--hud-ok)]">${coriolisSvg.replace('<svg ', '<svg class="w-full h-full" ')}</span>
      <span>4. BGS & Galaxia Viva</span>
    </button>
    <button class="nav-tab flex items-center gap-2" onclick="switchTab('onfoot')">
      <span>🥾</span>
      <span>5. A Pie (Odyssey)</span>
    </button>
  </div>`;

if (html.includes(navTabsTarget)) {
  html = html.replace(navTabsTarget, newNavTabs);
  console.log('Replaced nav-tab emojis with official ED vectors!');
}

// 3. Add Gutamaya manufacturer logo to Imperial Cutter in Colonisation
const cutterColonisationTarget = `Basado en tu nave activa: <strong class="text-white">Imperial Cutter (720 t)</strong>`;
const newCutterColonisation = `Basado en tu nave activa: 
              <span class="inline-flex items-center gap-1.5 ml-1">
                <span class="h-3 inline-block text-[var(--hud-cyan)]" title="Fabricante: Gutamaya">${gutamayaSvg.replace('<svg ', '<svg class="h-3 w-auto max-w-[80px]" ')}</span>
                <strong class="text-white">Imperial Cutter (720 t)</strong>
              </span>`;

if (html.includes(cutterColonisationTarget)) {
  html = html.replace(cutterColonisationTarget, newCutterColonisation);
  console.log('Added Gutamaya logo to Imperial Cutter in Colonisation tab!');
}

// 4. In BGS tab (Sistema Sol), add the Federation superpower insignia
const solHeaderTarget = `<h2 class="text-2xl font-black text-white glow-orange">
                Sistema Sol
              </h2>`;

const newSolHeader = `<div class="flex items-center gap-3">
                <div class="w-8 h-8 text-[var(--hud-primary)] filter drop-shadow-[0_0_8px_rgba(255,119,0,0.6)]" title="Superpotencia: Federación">
                  ${federationSvg.replace('<svg ', '<svg class="w-full h-full" ')}
                </div>
                <h2 class="text-2xl font-black text-white glow-orange">
                  Sistema Sol · Capital Federal
                </h2>
              </div>`;

if (html.includes(solHeaderTarget)) {
  html = html.replace(solHeaderTarget, newSolHeader);
  console.log('Added Federation insignia to Sol in BGS tab!');
}

fs.writeFileSync(mockupPath, html, 'utf8');
console.log('Updated mockup completely!');
