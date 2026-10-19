import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
const uiDir = path.resolve(scriptsDir, '..');
const port = Number(process.env.TMC_E2E_PORT ?? 47320);
const smokeDir = path.join(os.tmpdir(), `tmc-playwright-smoke-${process.pid}`);
const journalDir = path.join(smokeDir, 'journal');
const dataDir = path.join(smokeDir, 'data');
const configPath = path.join(smokeDir, 'config.toml');
const fixtureName = process.env.TMC_E2E_FIXTURE ?? 'real_explo_01.log';
const online = process.env.TMC_E2E_ONLINE === 'true';
const ardentPort = Number(process.env.TMC_E2E_ARDENT_PORT ?? 0);
let fixturePath = path.resolve(uiDir, '../tests/fixtures/journal', fixtureName);

// Desplaza las marcas de tiempo del diario para que el último evento sea de hace ~1 min:
// las edades de Ardent (datos recientes del servidor falso) quedan coherentes con el diario.
if (process.env.TMC_E2E_SHIFT === 'true') {
  const text = fs.readFileSync(fixturePath, 'utf8');
  const stamps = [...text.matchAll(/"timestamp":\s*"([^"]+)"/g)].map((m) => Date.parse(m[1]));
  const delta = Date.now() - 60_000 - Math.max(...stamps);
  const shifted = text.replace(/("timestamp":\s*")([^"]+)(")/g, (_, a, ts, c) =>
    `${a}${new Date(Date.parse(ts) + delta).toISOString().replace(/\.\d+Z$/, 'Z')}${c}`);
  fs.mkdirSync(smokeDir, { recursive: true });
  fixturePath = path.join(smokeDir, fixtureName);
  fs.writeFileSync(fixturePath, shifted);
}

// Ardent falso: solo en 127.0.0.1; devuelve compradores de platino con fechas relativas a ahora.
let fakeArdent = null;
if (ardentPort > 0) {
  const ago = (min) => new Date(Date.now() - min * 60_000).toISOString();
  const row = (name, type, pad, sys, addr, price, demand, min, dist) => ({
    commodityName: 'platinum', marketId: 9100000000 + addr, stationName: name, stationType: type,
    distanceToArrival: 100.0, maxLandingPadSize: pad, bodyId: null, bodyName: null,
    systemAddress: 9000000000000 + addr, systemName: sys, systemX: 0, systemY: 0, systemZ: 0,
    buyPrice: 0, demand, demandBracket: 3, meanPrice: 55000, sellPrice: price, stock: 0,
    stockBracket: 0, updatedAt: ago(min), distance: dist,
  });
  fakeArdent = http.createServer((req, res) => {
    const m = /\/commodity\/name\/([^/]+)\/nearby\/imports/.exec(req.url ?? '');
    const body = m && m[1] === 'platinum'
      ? [
          row('Ficticia Hub', 'Coriolis', 3, 'Testia Alfa', 1, 299000, 500, 30, 12),
          row('Prueba Outpost', 'Outpost', 2, 'Testia Beta', 2, 288000, 900, 180, 20),
          row('Puerto Pequeno', 'Outpost', 1, 'Testia Gamma', 3, 310000, 900, 10, 25),
        ]
      : [];
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify(body));
  });
  fakeArdent.listen(ardentPort, '127.0.0.1');
}


fs.mkdirSync(journalDir, { recursive: true });
fs.mkdirSync(dataDir, { recursive: true });
fs.writeFileSync(
  configPath,
  `data_dir = "${dataDir.replace(/\\/g, '/')}"\nauto_copy_next_system = false\nonline_services = ${online}\n[server]\nport = ${port}\nlan = false\n`,
);

const exeSuffix = process.platform === 'win32' ? '.exe' : '';
const replayBin = path.resolve(uiDir, `../target/debug/tmc-replay${exeSuffix}`);
const headlessBin = path.resolve(uiDir, `../target/debug/tmc-headless${exeSuffix}`);

const childEnv = { ...process.env };
if (ardentPort > 0) childEnv.TMC_E2E_ARDENT_BASE = `http://127.0.0.1:${ardentPort}`;

const headless = fs.existsSync(headlessBin)
  ? spawn(
      headlessBin,
      [
        '--config',
        configPath,
        '--journal-dir',
        journalDir,
        '--ui-dir',
        path.resolve(uiDir, 'dist'),
      ],
      { stdio: 'inherit', env: childEnv },
    )
  : spawn(
      'cargo',
      [
        'run',
        '--manifest-path',
        path.resolve(uiDir, '../Cargo.toml'),
        '-p',
        'tmc-headless',
        '--',
        '--config',
        configPath,
        '--journal-dir',
        journalDir,
        '--ui-dir',
        path.resolve(uiDir, 'dist'),
      ],
      { stdio: 'inherit', env: childEnv },
    );

setTimeout(() => {
  if (fs.existsSync(replayBin)) {
    spawn(replayBin, ['play', '--from', fixturePath, '--to', journalDir, '--speed', '10000'], {
      stdio: 'ignore',
    });
  } else {
    spawn(
      'cargo',
      [
        'run',
        '--manifest-path',
        path.resolve(uiDir, '../Cargo.toml'),
        '-p',
        'tmc-replay',
        '--',
        'play',
        '--from',
        fixturePath,
        '--to',
        journalDir,
        '--speed',
        '10000',
      ],
      { stdio: 'ignore' },
    );
  }
}, 500);

function cleanup() {
  fakeArdent?.close();
  headless.kill();
}

process.on('SIGINT', cleanup);
process.on('SIGTERM', cleanup);
headless.on('exit', (code) => {
  process.exit(code ?? 0);
});
