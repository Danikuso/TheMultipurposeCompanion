// Actual panels with synthetic state; never connects to the commander's live backend.
import { chromium } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const output = process.argv[2];
if (!output || !path.isAbsolute(output)) throw new Error('An absolute screenshot output directory is required');
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ channel: process.env.TMC_E2E_CHANNEL ?? 'msedge', headless: true });
const report = [], errors = [];
try {
  const page = await browser.newPage();
  page.on('pageerror', error => errors.push(error.message));
  const cases = [375, 1280, 1920].flatMap(width => ['es', 'en', 'fr', 'de', 'pt'].map(lang => ({ width, lang, narrow: false })));
  cases.push(...['es', 'en', 'fr', 'de', 'pt'].map(lang => ({ width: 1280, lang, narrow: true })));
  for (const { width, lang, narrow } of cases) {
    await page.setViewportSize({ width, height: 1100 });
    await page.goto(`http://127.0.0.1:47680/tests/fixtures/i18n-preview.html?lang=${lang}`);
    await page.locator('.hud-panel').first().waitFor();
    for (const button of await page.locator('.panel-header-toggle').all()) await button.click();
    if (narrow) await page.locator('.qa-panel').first().evaluate(element => { element.style.width = '300px'; });
    const bounds = await page.evaluate(() => {
      const bad = [];
      for (const panel of document.querySelectorAll('.hud-panel')) {
        const box = panel.getBoundingClientRect();
        for (const element of panel.querySelectorAll('button,input,select,.badge,.rank-current-badge,h2')) {
          const rect = element.getBoundingClientRect();
          if (rect.width && rect.height && (rect.left < box.left - 1 || rect.right > box.right + 1)) {
            bad.push({ text: element.textContent, left: rect.left, right: rect.right, panel: box.width });
          }
        }
      }
      for (const badge of document.querySelectorAll('.rank-current-badge, .rank-quant-val, .rank-pct')) {
        const cell = badge.closest('.rank-card');
        if (!cell) continue;
        const rect = badge.getBoundingClientRect(), box = cell.getBoundingClientRect();
        if (rect.left < box.left - 1 || rect.right > box.right + 1) bad.push({ text: badge.textContent, left: rect.left, right: rect.right, panel: box.width });
      }
      return { bad, horizontal: document.documentElement.scrollWidth > innerWidth + 1 };
    });
    report.push({ lang, width, narrow, ...bounds });
    await (narrow ? page.locator('.qa-panel').first() : page.locator('#preview')).screenshot({ path: path.join(output, narrow ? `commander-${lang}-300.png` : `panels-${lang}-${width}.png`) });
  }
  fs.writeFileSync(path.join(output, 'panels-report.json'), JSON.stringify({ report, errors }, null, 2));
  const bad = report.filter(row => row.bad.length || row.horizontal);
  if (errors.length || bad.length) throw new Error(JSON.stringify({ errors, bad }));
  console.log(`PASS: ${report.length} panel views; five languages; three widths; no out-of-frame controls`);
} finally { await browser.close(); }
