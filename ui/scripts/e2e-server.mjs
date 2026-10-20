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
const relayPort = Number(process.env.TMC_E2E_RELAY_PORT ?? 0);
const relayFile = process.env.TMC_E2E_RELAY_FILE ?? '';
const autoCopy = process.env.TMC_E2E_AUTOCOPY === 'true';
const withMarket = process.env.TMC_E2E_MARKET === 'true';
let fixturePath = path.resolve(uiDir, '../tests/fixtures/journal', fixtureName);
let shiftDelta = 0;

// Desplaza las marcas de tiempo (y caducidades) del diario para que el último evento sea de
// hace ~1 min: las edades de los servicios falsos quedan coherentes con el diario.
if (process.env.TMC_E2E_SHIFT === 'true') {
  const text = fs.readFileSync(fixturePath, 'utf8');
  const stamps = [...text.matchAll(/"timestamp":\s*"([^"]+)"/g)].map((m) => Date.parse(m[1]));
  const delta = Date.now() - 60_000 - Math.max(...stamps);
  shiftDelta = delta;
  const shifted = text.replace(/("(?:timestamp|Expiry)":\s*")([^"]+)(")/g, (_, a, ts, c) =>
    `${a}${new Date(Date.parse(ts) + delta).toISOString().replace(/\.\d+Z$/, 'Z')}${c}`);
  fs.mkdirSync(smokeDir, { recursive: true });
  fixturePath = path.join(smokeDir, fixtureName);
  fs.writeFileSync(fixturePath, shifted);
}

fs.mkdirSync(journalDir, { recursive: true });
fs.mkdirSync(dataDir, { recursive: true });

// Servidor falso local (solo 127.0.0.1): Ardent, Spansh y EDSM, y POST /__journal para
// anexar líneas al diario en curso desde las pruebas.
let fakeServer = null;
if (ardentPort > 0) {
  const ago = (min) => new Date(Date.now() - min * 60_000).toISOString();
  const row = (commodity, name, type, pad, sys, addr, price, demand, min, dist) => ({
    commodityName: commodity, marketId: 9100000000 + addr, stationName: name, stationType: type,
    distanceToArrival: 100.0, maxLandingPadSize: pad, bodyId: null, bodyName: null,
    systemAddress: 9000000000000 + addr, systemName: sys, systemX: 0, systemY: 0, systemZ: 0,
    buyPrice: 0, demand, demandBracket: 3, meanPrice: 55000, sellPrice: price, stock: 0,
    stockBracket: 0, updatedAt: ago(min), distance: dist,
  });
  const fixtures = path.resolve(uiDir, '../tests/fixtures/net');
  // Turismo: 3 destinos (tour a/b/c) más el origen de la fixture; solo los destinos necesitan coordenadas aquí.
  const tourCoords = { 'tour a': [10, 0, 0], 'tour b': [-10, 0, 0], 'tour c': [11, 0, 0] };
  const send = (res, status, body) => {
    res.statusCode = status;
    res.setHeader('content-type', 'application/json');
    res.end(typeof body === 'string' ? body : JSON.stringify(body));
  };
  fakeServer = http.createServer((req, res) => {
    const url = req.url ?? '';
    if (req.method === 'POST' && url.startsWith('/__journal')) {
      let data = '';
      req.on('data', (c) => (data += c));
      req.on('end', () => {
        const logs = fs.readdirSync(journalDir).filter((f) => f.endsWith('.log')).sort();
        if (logs.length === 0) return send(res, 409, { error: 'no journal yet' });
        fs.appendFileSync(path.join(journalDir, logs[logs.length - 1]), data.trimEnd() + '\n');
        send(res, 200, { ok: true });
      });
      return;
    }
    if (req.method === 'POST' && url.startsWith('/api/trade/route')) {
      req.resume();
      return send(res, 200, fs.readFileSync(path.join(fixtures, 'spansh_trade_job.json'), 'utf8'));
    }
    if (url.startsWith('/api/results/')) {
      return send(res, 200, fs.readFileSync(path.join(fixtures, 'spansh_trade_result.json'), 'utf8'));
    }
    const sys = /\/system\/name\/([^/?]+)/.exec(url);
    if (sys) {
      const name = decodeURIComponent(sys[1]);
      const c = tourCoords[name.toLowerCase()];
      return c
        ? send(res, 200, { systemAddress: 9500000000000 + c[0], systemName: name, systemX: c[0], systemY: c[1], systemZ: c[2] })
        : send(res, 404, { error: 'Not Found' });
    }
    const m = /\/commodity\/name\/([^/]+)\/nearby\/imports/.exec(url);
    let body = [];
    if (m && m[1] === 'platinum') {
      body = [
        row('platinum', 'Ficticia Hub', 'Coriolis', 3, 'Testia Alfa', 1, 299000, 500, 30, 12),
        row('platinum', 'Prueba Outpost', 'Outpost', 2, 'Testia Beta', 2, 288000, 900, 180, 20),
        row('platinum', 'Puerto Pequeno', 'Outpost', 1, 'Testia Gamma', 3, 310000, 900, 10, 25),
      ];
    } else if (m && m[1] === 'gold') {
      // Reciente y compatible (L), antigua de 49 h (excluida) y de plataforma pequeña (excluida).
      body = [
        row('gold', 'Ficticia Hub', 'Coriolis', 3, 'Testia Alfa', 1, 11000, 500, 30, 12),
        row('gold', 'Puerto Antiguo', 'Coriolis', 3, 'Testia Beta', 2, 15000, 900, 49 * 60, 20),
        row('gold', 'Puerto Pequeno', 'Outpost', 1, 'Testia Gamma', 3, 14000, 900, 10, 25),
      ];
    }
    send(res, 200, body);
  });
  fakeServer.listen(ardentPort, '127.0.0.1');
}

fs.writeFileSync(
  configPath,
  `data_dir = "${dataDir.replace(/\\/g, '/')}"\nauto_copy_next_system = ${autoCopy}\nonline_services = ${online}\n[server]\nport = ${port}\nlan = false\n`,
);

const exeSuffix = process.platform === 'win32' ? '.exe' : '';
const replayBin = path.resolve(uiDir, `../target/debug/tmc-replay${exeSuffix}`);
const headlessBin = path.resolve(uiDir, `../target/debug/tmc-headless${exeSuffix}`);

// Solo los escenarios de F3 (comercio/misiones/EDDN) simulan «juego en marcha».
const f3Scenario = ['trade_session.log', 'missions_session.log'].includes(fixtureName);
const childEnv = { ...process.env };
if (f3Scenario) childEnv.TMC_E2E_GAME_RUNNING = '1';
else delete childEnv.TMC_E2E_GAME_RUNNING;
if (relayPort > 0) childEnv.TMC_E2E_EDDN_RELAY = `tcp://127.0.0.1:${relayPort}`;
if (ardentPort > 0) childEnv.TMC_E2E_ARDENT_BASE = `http://127.0.0.1:${ardentPort}`;

const headless = fs.existsSync(headlessBin)
  ? spawn(
      headlessBin,
      ['--config', configPath, '--journal-dir', journalDir, '--ui-dir', path.resolve(uiDir, 'dist')],
      { stdio: 'inherit', env: childEnv },
    )
  : spawn(
      'cargo',
      [
        'run', '--manifest-path', path.resolve(uiDir, '../Cargo.toml'), '-p', 'tmc-headless', '--',
        '--config', configPath, '--journal-dir', journalDir, '--ui-dir', path.resolve(uiDir, 'dist'),
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
        'run', '--manifest-path', path.resolve(uiDir, '../Cargo.toml'), '-p', 'tmc-replay', '--',
        'play', '--from', fixturePath, '--to', journalDir, '--speed', '10000',
      ],
      { stdio: 'ignore' },
    );
  }
}, 500);

// Market.json (marcas de tiempo desplazadas) una vez arrancada la reproducción.
if (withMarket) {
  setTimeout(() => {
    const raw = fs.readFileSync(path.resolve(uiDir, '../tests/fixtures/companion/market_trade.json'), 'utf8');
    const out = raw.replace(/("timestamp":\s*")([^"]+)(")/, (_, a, ts, c) =>
      `${a}${new Date(Date.parse(ts) + shiftDelta).toISOString().replace(/\.\d+Z$/, 'Z')}${c}`);
    fs.writeFileSync(path.join(journalDir, 'Market.json'), out);
    // El evento `Market` del diario avisa de que el archivo está listo.
    setTimeout(() => {
      const logs = fs.readdirSync(journalDir).filter((f) => f.endsWith('.log')).sort();
      if (logs.length === 0) return;
      const line = JSON.stringify({
        timestamp: new Date().toISOString().replace(/\.\d+Z$/, 'Z'), event: 'Market',
        MarketID: 128000001, StationName: 'Synthetic Port', StationType: 'Coriolis', StarSystem: 'Synthetic System',
      });
      fs.appendFileSync(path.join(journalDir, logs[logs.length - 1]), line + '\n');
    }, 500);
  }, 3000);
}

// Relay EDDN falso (ZeroMQ PUB local) con el ejemplo de tmc-eddn.
let relay = null;
if (relayPort > 0 && relayFile) {
  const relayBin = path.resolve(uiDir, `../target/debug/examples/fake_relay${exeSuffix}`);
  const file = path.resolve(uiDir, '../tests/fixtures/eddn', relayFile);
  relay = fs.existsSync(relayBin)
    ? spawn(relayBin, [String(relayPort), file], { stdio: 'ignore' })
    : spawn(
        'cargo',
        ['run', '--manifest-path', path.resolve(uiDir, '../Cargo.toml'), '-p', 'tmc-eddn', '--example', 'fake_relay', '--', String(relayPort), file],
        { stdio: 'ignore' },
      );
}

function cleanup() {
  relay?.kill();
  fakeServer?.close();
  headless.kill();
}

process.on('SIGINT', cleanup);
process.on('SIGTERM', cleanup);
headless.on('exit', (code) => {
  process.exit(code ?? 0);
});
