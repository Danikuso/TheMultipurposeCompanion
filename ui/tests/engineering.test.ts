import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { live, disconnect } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MaterialsPanel from '../src/modules/engineering/MaterialsPanel.svelte';
import fixture from './fixtures/engineering-topics.json';
import { createPageReader } from '../src/modules/engineering/pages.svelte';
let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;
beforeEach(() => {
 disconnect(); setLocale('es'); live.connected = true;
 for (const [key, value] of Object.entries(structuredClone(fixture))) if (key !== 'engineering.pages') live.topics[key] = value;
 live.topics['core.cmdr'] = { name: 'Synthetic' };
 target = document.createElement('div'); document.body.append(target);
 vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(fixture['engineering.pages'].locker_components))));
});
afterEach(async () => { if (mounted) await unmount(mounted); mounted = undefined; target.remove(); vi.unstubAllGlobals(); });
function show() { mounted = mount(MaterialsPanel, { target }); flushSync(); }
function click(id: string) { (target.querySelector(`[data-testid="${id}"]`) as HTMLButtonElement).click(); flushSync(); }
async function settle() { await new Promise(resolve => setTimeout(resolve, 0)); flushSync(); }
test('materials grouped by grade with count over max', () => { show(); expect(target.textContent).toContain('G1'); expect(target.querySelector('[data-key="iron"]')?.textContent).toContain('117 / 300'); expect(fetch).not.toHaveBeenCalled(); });
test('at max warning visible', () => { show(); expect(target.textContent).toContain('Al máximo'); });
test('unknown material shows journal name and no data', () => { const inventory = structuredClone(fixture['engineering.inventory']); Object.assign(inventory.by_category.Encoded['0'][0], { name_localised: 'Nuevo material' }); live.topics['engineering.inventory'] = inventory; show(); click('tab-Encoded'); expect(target.textContent).toContain('Nuevo material'); expect(target.textContent).toContain('Sin datos'); });
test('material name falls back to name_en', () => { const inventory = structuredClone(fixture['engineering.inventory']); Object.assign(inventory.by_category.Raw['1'][1], { name_localised: null, name_en: 'Iron' }); live.topics['engineering.inventory'] = inventory; show(); expect(target.textContent).toContain('Iron'); });
test('locker tab lists components', async () => { show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Graphene'); expect(target.querySelector('[data-key="graphene"]')?.textContent).toContain('8'); expect(fetch).toHaveBeenCalledWith('/api/engineering/locker?revision=17&cursor=0&section=components', expect.objectContaining({ cache: 'no-store', signal: expect.any(AbortSignal) })); });
test('session totals by origin', () => { show(); expect(target.querySelector('[data-origin="Scan"]')?.textContent).toContain('+2'); expect(target.querySelector('[data-origin="Trade"]')?.textContent).toContain('+1 / −6'); });
test('reader accumulates all pages atomically', async () => {
 const page = fixture['engineering.pages'].locker_components;
 let finish!: (value: Response) => void;
 vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({ ...page, next_cursor: 1, total: 2 }))).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; })));
 const reader = createPageReader<{ key: string }>(); const pending = reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' });
 await settle(); expect(reader.items).toEqual([]); finish(new Response(JSON.stringify({ ...page, cursor: 1, total: 2, items: [{ key: 'second' }] }))); await pending;
 expect(reader.items.map(row => row.key)).toEqual(['graphene', 'second']); expect(reader.state).toBe('ready'); reader.clear();
});
test('late response cannot resurrect cleared rows even if abort ignored', async () => {
 let finish!: (value: Response) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; })));
 const reader = createPageReader<unknown>(); const pending = reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' }); reader.clear(); finish(new Response(JSON.stringify(fixture['engineering.pages'].locker_components))); await pending;
 expect(reader.items).toEqual([]); expect(reader.state).toBe('idle');
});
test('409 is stale without retry', async () => { vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 409 }))); const reader = createPageReader<unknown>(); await reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' }); expect(reader.state).toBe('stale'); expect(fetch).toHaveBeenCalledTimes(1); });
test('reader rejects mismatched revision, section, total and non-progressing cursors', async () => {
 const base = fixture['engineering.pages'].locker_components;
 for (const change of [{ revision: 18 }, { section: 'data' }, { total: 2 }, { next_cursor: 0 }]) {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ ...base, ...change }))));
  const reader = createPageReader<unknown>(); await reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' }); expect(reader.state).toBe('failed'); expect(reader.items).toEqual([]); expect(fetch).toHaveBeenCalledTimes(1);
 }
});
test('two readers have independent cursors and rows', async () => {
 const readerA = createPageReader<{ key: string }>(); const readerB = createPageReader<{ key: string }>();
 await readerA.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' });
 await readerB.load({ resource: 'locker', section: 'components', revision: 17, commander: 'B' });
 readerA.clear(); expect(readerA.items).toEqual([]); expect(readerB.items[0].key).toBe('graphene'); readerB.clear();
});
test('section, revision and commander changes clear previous rows', async () => {
 show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Graphene');
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
 const select = target.querySelector('select')!; select.value = 'data'; select.dispatchEvent(new Event('change', { bubbles: true })); flushSync(); expect(target.textContent).not.toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(1);
 live.topics['engineering.locker'] = { ...fixture['engineering.locker'], revision: 18 }; flushSync(); expect(target.textContent).not.toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(2);
 live.topics['core.cmdr'] = { name: 'Other' }; flushSync(); expect(target.textContent).not.toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(3);
});
test('as_of-only WS updates do not restart REST', async () => {
 show(); click('tab-locker'); await settle(); vi.mocked(fetch).mockClear();
 live.topics['engineering.locker'] = { ...fixture['engineering.locker'], as_of: '2030-01-01T00:00:00Z' }; flushSync(); await settle(); expect(fetch).not.toHaveBeenCalled(); expect(target.textContent).toContain('Graphene');
});
test('leaving locker and unmount abort pending request', async () => {
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {}))); show(); click('tab-locker');
 const firstSignal = vi.mocked(fetch).mock.calls[0][1]!.signal!; click('tab-Raw'); expect(firstSignal.aborted).toBe(true); expect(fetch).toHaveBeenCalledTimes(1);
 click('tab-locker'); const secondSignal = vi.mocked(fetch).mock.calls[1][1]!.signal!; await unmount(mounted!); mounted = undefined; expect(secondSignal.aborted).toBe(true);
});
test('hidden document cancels REST and visible document reloads', async () => {
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {}))); show(); click('tab-locker'); const signal = vi.mocked(fetch).mock.calls[0][1]!.signal!;
 const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true); document.dispatchEvent(new Event('visibilitychange')); flushSync(); expect(signal.aborted).toBe(true); expect(fetch).toHaveBeenCalledTimes(1);
 hidden.mockReturnValue(false); document.dispatchEvent(new Event('visibilitychange')); flushSync(); expect(fetch).toHaveBeenCalledTimes(2); hidden.mockRestore();
});
test('disconnect clears rows and reconnect reloads same revision', async () => {
 show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Graphene');
 live.connected = false; flushSync(); expect(target.textContent).not.toContain('Graphene'); live.connected = true; flushSync(); await settle(); expect(target.textContent).toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(2);
});
test('REST failure is localized and does not retry', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 500 }))); show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('No se pudieron cargar los datos'); expect(fetch).toHaveBeenCalledTimes(1);
});
test('409 panel waits for new WS revision', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 409 }))); show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Esperando el inventario actualizado'); await settle(); expect(fetch).toHaveBeenCalledTimes(1);
 live.topics['engineering.locker'] = { ...fixture['engineering.locker'], revision: 18 }; flushSync(); await settle(); expect(fetch).toHaveBeenCalledTimes(2);
});
test('two mounted panels keep independent locker readers', async () => {
 show(); click('tab-locker'); await settle();
 const secondTarget = document.createElement('div'); document.body.append(secondTarget);
 const second = mount(MaterialsPanel, { target: secondTarget }); flushSync();
 (secondTarget.querySelector('[data-testid="tab-locker"]') as HTMLButtonElement).click(); flushSync(); await settle();
 try { expect(fetch).toHaveBeenCalledTimes(2); expect(secondTarget.textContent).toContain('Graphene'); click('tab-Raw'); expect(target.textContent).not.toContain('Graphene'); expect(secondTarget.textContent).toContain('Graphene'); }
 finally { await unmount(second); secondTarget.remove(); }
});
test('late JSON parsing cannot publish rows after another commander starts', async () => {
 let finish!: (value: unknown) => void;
 vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce({ ok: true, status: 200, json: () => new Promise(resolve => { finish = resolve; }) }).mockResolvedValueOnce(new Response(JSON.stringify({ ...fixture['engineering.pages'].locker_components, items: [{ key: 'new', count: 1, name_localised: null }] }))));
 const reader = createPageReader<{ key: string }>(); const first = reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'A' });
 await settle(); await reader.load({ resource: 'locker', section: 'components', revision: 17, commander: 'B' }); finish(fixture['engineering.pages'].locker_components); await first;
 expect(reader.items.map(row => row.key)).toEqual(['new']); expect(reader.state).toBe('ready'); reader.clear();
});
import EngineersPanel from '../src/modules/engineering/EngineersPanel.svelte';
import type { EngineerNextStep } from '../src/modules/engineering/types';
import { blueprintLabel } from '../src/modules/engineering/labels';
function showEngineers() {
 mounted = mount(EngineersPanel, { target });
 flushSync();
}
function selectEngineer(next_step: EngineerNextStep = 'Done', blueprint = 'engine_dirty') {
 live.topics['engineering.engineers'] = {
  ...fixture['engineering.engineers'],
  selected: { id: 300100, next_step, blueprints: [{ blueprint, max_grade: 5 }] },
 };
}
test('engineer row shows status rank progress system distance and verification', () => {
 showEngineers();
 const row = target.querySelector('[data-engineer-id="300100"]')!;
 expect(row.textContent).toContain('Felicity Farseer');
 expect(row.textContent).toContain('Desbloqueado');
 expect(row.textContent).toContain('5');
 expect(row.textContent).toContain('0 %');
 expect(row.textContent).toContain('Deciat');
 expect(row.textContent).toContain('12,3 al');
 expect(row.textContent).toContain('Sin verificar');
});
test('select engineer posts json action', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}')));
 showEngineers();
 (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 await settle();
 expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.select_engineer', {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ id: 300100 }),
 });
});
test('next step contribution shows have and need', () => {
 selectEngineer({ Contribute: { item: 'iron', kind: 'material', have: 4, need: 10 } });
 showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('4 / 10');
 expect(target.textContent).toContain('Hierro');
});
test('on foot engineers in their own section', () => {
 showEngineers();
 expect(target.querySelector('[data-testid="engineers-on-foot"]')?.textContent).toContain('Domino Green');
 expect(target.querySelector('[data-testid="engineers-ship"]')?.textContent).not.toContain('Domino Green');
});
test('engineer blueprints with official name and max grade', () => {
 selectEngineer(); showEngineers();
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('Modificación de Motor Sucio hasta G5');
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).not.toContain('Traducción pendiente');
});
test('all next step variants are rendered without raw Fluent keys', () => {
 selectEngineer({ Requirement: { key: 'unlock-felicity-farseer' } }); showEngineers();
 expect(target.textContent).toContain('Cumple el requisito');
 selectEngineer({ Invite: { key: 'unlock-invite-felicity-farseer' } }); flushSync();
 expect(target.textContent).toContain('Consigue la invitación');
 selectEngineer('Done'); flushSync();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Desbloqueado');
});
test('unknown status null rank progress and distance show localized no data', () => {
 const engineers = structuredClone(fixture['engineering.engineers']);
 Object.assign(engineers.engineers[0], { status: 'FutureStatus', rank: null, rank_progress: null, distance_ly: null });
 live.topics['engineering.engineers'] = engineers; showEngineers();
 const row = target.querySelector('[data-engineer-id="300100"]')!;
 expect(row.textContent).toContain('Desconocido'); expect(row.textContent).toContain('Sin datos');
 expect(row.textContent).not.toContain('FutureStatus'); expect(row.textContent).not.toContain('null');
});
test('unknown blueprint uses English fallback and marks translation pending', () => {
 expect(blueprintLabel('unknown_blueprint', 'New Drive Tuning')).toEqual({ name: 'New Drive Tuning', pending: true });
 selectEngineer('Done', 'unknown_blueprint'); showEngineers();
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('Traducción pendiente');
});
test('engineer selection failure shows localized error', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 500 })));
 showEngineers(); (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 await settle(); expect(target.textContent).toContain('No se pudo seleccionar el ingeniero');
});
test('missing engineers topic renders no data', () => {
 delete live.topics['engineering.engineers']; showEngineers();
 expect(target.textContent).toContain('Sin datos');
});
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { translationPending } from '../src/lib/i18n';
test('pending Fluent attribute is read from a real compiled resource', () => {
 const bundle = new FluentBundle('es');
 bundle.addResource(new FluentResource('bp-synthetic = English Drive Tuning\n    .pending = 1'));
 const original = FluentBundle.prototype.getMessage;
 const spy = vi.spyOn(FluentBundle.prototype, 'getMessage').mockImplementation(function (this: FluentBundle, key: string) {
  return key === 'bp-synthetic' ? original.call(bundle, key) : original.call(this, key);
 });
 try {
  expect(blueprintLabel('synthetic')).toEqual({ name: 'English Drive Tuning', pending: true });
  expect(translationPending('bp-engine_dirty')).toBe(false);
  selectEngineer('Done', 'synthetic'); showEngineers();
  expect(target.textContent).toContain('English Drive Tuning');
  expect(target.textContent).toContain('Traducción pendiente');
 } finally { spy.mockRestore(); }
});
test('obsolete engineer selection failure cannot overwrite a newer successful selection', async () => {
 let rejectFirst!: (error: Error) => void;
 vi.stubGlobal('fetch', vi.fn()
  .mockImplementationOnce(() => new Promise<Response>((_, reject) => { rejectFirst = reject; }))
  .mockResolvedValueOnce(new Response('{}')));
 showEngineers();
 (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 (target.querySelector('[data-engineer-id="300160"] button') as HTMLButtonElement).click();
 await settle(); rejectFirst(new Error('Unavailable')); await settle();
 expect(target.textContent).not.toContain('No se pudo seleccionar el ingeniero');
});
test('zero commodity contribution has readable catalog name and pending translation', () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 0, need: 3 } });
 showEngineers();
 const step = target.querySelector('[data-testid="engineer-next-step"]')!;
 expect(step.textContent).toContain('Soontill Relics');
 expect(step.textContent).toContain('0 / 3');
 expect(step.textContent).toContain('Traducción pendiente');
 expect(step.textContent).not.toContain('soontillrelics');
});
test('commodity contribution uses core cargo journal name and updates reactively', () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 2, need: 3 } });
 live.topics['core.ship'] = { cargo: [{ Name: 'soontillrelics', Name_Localised: 'Nombre del diario', Count: 2 }] };
 showEngineers();
 const step = target.querySelector('[data-testid="engineer-next-step"]')!;
 expect(step.textContent).toContain('Nombre del diario'); expect(step.textContent).toContain('2 / 3');
 expect(step.textContent).not.toContain('Traducción pendiente');
 live.topics['core.ship'] = { cargo: [] }; setLocale('en'); flushSync();
 expect(step.textContent).toContain('Soontill Relics'); expect(step.textContent).not.toContain('Translation pending');
});
test('commodity contribution uses trade cargo localized name when available', () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 2, need: 3 } });
 live.topics['trade.cargo'] = { lines: [{ commodity: 'soontillrelics', name_localised: 'Mercancía del diario', tons: 2 }] };
 showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Mercancía del diario');
});
test('credit contribution uses localized credits and backend counts', () => {
 selectEngineer({ Contribute: { item: 'credits', kind: 'credits', have: 4, need: 10 } });
 showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Créditos');
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('4 / 10');
});
