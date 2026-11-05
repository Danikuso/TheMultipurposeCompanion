// Visual QA of the isolated fixture, never the commander's live session.
import { chromium } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const output = process.argv[2];
if (!output || !path.isAbsolute(output)) throw new Error('An absolute screenshot output directory is required');
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ channel: process.env.TMC_E2E_CHANNEL ?? 'msedge', headless: true });
const report = [];
try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => Object.defineProperty(navigator, 'clipboard', { value: { writeText: async () => {} }, configurable: true }));
  const cases = [];
  for (const width of [375, 1280, 1920]) for (const lang of ['es', 'en', 'fr', 'de', 'pt']) {
    cases.push({ name: `${lang}-${width}`, width, query: `lang=${lang}&id=300010&status=Known`, expanded: true });
  }
  for (const scenario of [
    ['invited', 'id=300100&status=Invited'], ['rank-3', 'id=300100&status=Unlocked&rank=3&progress=42'],
    ['rank-5', 'id=300100&status=Unlocked&rank=5'], ['odyssey', 'id=400002&status=Unlocked'],
    ['barred', 'id=300100&status=Barred'], ['unknown', 'id=300000&status=Unknown'],
  ]) cases.push({ name: scenario[0], width: 1280, query: scenario[1], expanded: false });
  for (const item of cases) {
    await page.setViewportSize({ width: item.width, height: 1000 });
    await page.goto(`http://127.0.0.1:47680/tests/fixtures/engineer-guide-preview.html?${item.query}`);
    await page.getByTestId('guide-select').waitFor();
    await page.getByTestId(item.name.startsWith('rank-') ? 'guide-rank' : item.name === 'odyssey' ? 'guide-select' : 'guide-next').waitFor();
    await page.locator('#preview').evaluate(el => { el.style.width = `${Math.min(980, window.innerWidth)}px`; });
    if (item.expanded) await page.locator('.panel-header-toggle').click();
    await page.mouse.move(0, 0);
    const bounds = await page.locator('.engineer-guide-panel').evaluate(panel => {
      const boundary = panel.getBoundingClientRect();
      const overflow = [];
      for (const el of panel.querySelectorAll('button, select, h2, h3, .stage, .metric, .next-action, .contacts li')) {
        const rect = el.getBoundingClientRect();
        if (rect.height === 0 || rect.width === 0) continue;
        if (rect.left < boundary.left - 1 || rect.right > boundary.right + 1) overflow.push(el.className || el.tagName);
      }
      return { width: boundary.width, overflow, horizontal: panel.scrollWidth > panel.clientWidth + 1 };
    });
    if (bounds.overflow.length || bounds.horizontal) throw new Error(`${item.name}: clipping ${JSON.stringify(bounds)}`);
    await page.locator('#preview').screenshot({ path: path.join(output, `guide-${item.name}.png`) });
    report.push({ ...item, ...bounds });
  }
  if (errors.length) throw new Error(errors.join('\n'));
  fs.writeFileSync(path.join(output, 'guide-visual-report.json'), JSON.stringify({ passed: report.length, errors, report }, null, 2));
  console.log(`PASS: ${report.length} visual cases; 375/1280/1920; five languages; no horizontal overflow or out-of-frame controls`);
} finally { await browser.close(); }
