import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = path.dirname(fileURLToPath(import.meta.url));
const uiDir = path.resolve(scriptsDir, '..');
const smokeDir = path.join(os.tmpdir(), `tmc-playwright-smoke-${process.pid}`);
const journalDir = path.join(smokeDir, 'journal');
const dataDir = path.join(smokeDir, 'data');
const configPath = path.join(smokeDir, 'config.toml');
const fixturePath = path.resolve(uiDir, '../tests/fixtures/journal/basic_session.log');

fs.mkdirSync(journalDir, { recursive: true });
fs.mkdirSync(dataDir, { recursive: true });
fs.writeFileSync(
  configPath,
  `data_dir = "${dataDir.replace(/\\/g, '/')}"\nauto_copy_next_system = false\n[server]\nport = 47320\nlan = false\n`,
);

const exeSuffix = process.platform === 'win32' ? '.exe' : '';
const replayBin = path.resolve(uiDir, `../target/debug/tmc-replay${exeSuffix}`);
const headlessBin = path.resolve(uiDir, `../target/debug/tmc-headless${exeSuffix}`);

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
      { stdio: 'inherit' },
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
      { stdio: 'inherit' },
    );

setTimeout(() => {
  if (fs.existsSync(replayBin)) {
    spawn(replayBin, ['play', '--from', fixturePath, '--to', journalDir, '--speed', '50'], {
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
        '50',
      ],
      { stdio: 'ignore' },
    );
  }
}, 500);

function cleanup() {
  headless.kill();
}

process.on('SIGINT', cleanup);
process.on('SIGTERM', cleanup);
headless.on('exit', (code) => {
  process.exit(code ?? 0);
});
