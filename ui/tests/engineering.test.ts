import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { live, disconnect } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import MaterialsPanel from '../src/modules/engineering/MaterialsPanel.svelte';
import fixture from './fixtures/engineering-topics.json';
import { createPageReader } from '../src/modules/engineering/pages.svelte';
import ShipPanel from '../src/modules/engineering/ShipPanel.svelte';
import EngineerFlow from '../src/modules/engineering/EngineerFlow.svelte';
import { engineeringActions } from '../src/modules/engineering/actions.svelte';
let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;
beforeEach(() => {
 engineeringActions.pending = false; engineeringActions.selectedSlot = null;
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
test('locker tab lists components', async () => { show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Graphene'); expect(target.querySelector('[data-key="graphene"]')?.textContent).toContain('8'); expect(fetch).toHaveBeenCalledWith('/api/engineering/locker?revision=21&cursor=0&section=components', expect.objectContaining({ cache: 'no-store', signal: expect.any(AbortSignal) })); });
test('session totals by origin', () => { show(); expect(target.querySelector('[data-origin="Scan"]')?.textContent).toContain('+2'); expect(target.querySelector('[data-origin="Trade"]')?.textContent).toContain('+1 / −6'); });
test('reader accumulates all pages atomically', async () => {
 const page = fixture['engineering.pages'].locker_components;
 let finish!: (value: Response) => void;
 vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({ ...page, next_cursor: 1, total: 2 }))).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; })));
 const reader = createPageReader<{ key: string }>(); const pending = reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' });
 await settle(); expect(reader.items).toEqual([]); finish(new Response(JSON.stringify({ ...page, cursor: 1, total: 2, items: [{ key: 'second' }] }))); await pending;
 expect(reader.items.map(row => row.key)).toEqual(['graphene', 'second']); expect(reader.state).toBe('ready'); reader.clear();
});
test('late response cannot resurrect cleared rows even if abort ignored', async () => {
 let finish!: (value: Response) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; })));
 const reader = createPageReader<unknown>(); const pending = reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' }); reader.clear(); finish(new Response(JSON.stringify(fixture['engineering.pages'].locker_components))); await pending;
 expect(reader.items).toEqual([]); expect(reader.state).toBe('idle');
});
test('409 is stale without retry', async () => { vi.stubGlobal('fetch', vi.fn(async () => new Response('', { status: 409 }))); const reader = createPageReader<unknown>(); await reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' }); expect(reader.state).toBe('stale'); expect(fetch).toHaveBeenCalledTimes(1); });
test('reader rejects mismatched revision, section, total and non-progressing cursors', async () => {
 const base = fixture['engineering.pages'].locker_components;
 for (const change of [{ revision: 22 }, { section: 'data' }, { total: 2 }, { next_cursor: 0 }]) {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ ...base, ...change }))));
  const reader = createPageReader<unknown>(); await reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' }); expect(reader.state).toBe('failed'); expect(reader.items).toEqual([]); expect(fetch).toHaveBeenCalledTimes(1);
 }
});
test('two readers have independent cursors and rows', async () => {
 const readerA = createPageReader<{ key: string }>(); const readerB = createPageReader<{ key: string }>();
 await readerA.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' });
 await readerB.load({ resource: 'locker', section: 'components', revision: 21, commander: 'B' });
 readerA.clear(); expect(readerA.items).toEqual([]); expect(readerB.items[0].key).toBe('graphene'); readerB.clear();
});
test('section, revision and commander changes clear previous rows', async () => {
 show(); click('tab-locker'); await settle(); expect(target.textContent).toContain('Graphene');
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
 const select = target.querySelector('select')!; select.value = 'data'; select.dispatchEvent(new Event('change', { bubbles: true })); flushSync(); expect(target.textContent).not.toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(1);
 live.topics['engineering.locker'] = { ...fixture['engineering.locker'], revision: 22 }; flushSync(); expect(target.textContent).not.toContain('Graphene'); expect(fetch).toHaveBeenCalledTimes(2);
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
 live.topics['engineering.locker'] = { ...fixture['engineering.locker'], revision: 22 }; flushSync(); await settle(); expect(fetch).toHaveBeenCalledTimes(2);
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
 const reader = createPageReader<{ key: string }>(); const first = reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'A' });
 await settle(); await reader.load({ resource: 'locker', section: 'components', revision: 21, commander: 'B' }); finish(fixture['engineering.pages'].locker_components); await first;
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
function showShip() { mounted = mount(ShipPanel, { target }); flushSync(); }
function showFlow() { mounted = mount(EngineerFlow, { target }); flushSync(); }
function change(id: string, value: string) { const input = target.querySelector(`[data-testid="${id}"]`) as HTMLInputElement; input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); input.dispatchEvent(new Event('change', { bubbles: true })); flushSync(); }
function actionBody(name: string) { return vi.mocked(fetch).mock.calls.find(([url]) => url === `/api/actions/engineering.${name}`)?.[1]?.body; }
test('ship shows stock and official current engineering including unsupported modules', async () => {
 const ship = structuredClone(fixture['engineering.ship']); ship.modules.push({ ...ship.modules[1], slot: 'Unsupported', engineerable: false }); live.topics['engineering.ship'] = ship;
 showShip(); expect(target.textContent).toContain('De serie'); expect(target.textContent).toContain('Aumento de Alcance de MDD'); expect(target.textContent).toContain('100 %');
 expect(target.querySelector('[data-slot="Unsupported"] button')).toBeNull();
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}'))); click('module-FrameShiftDrive'); await settle(); expect(actionBody('select_module')).toBe(JSON.stringify({ slot: 'FrameShiftDrive' }));
});
test('preview posts exact request and pin and close post empty objects', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}'))); showFlow(); click('preview'); await settle();
 expect(actionBody('preview')).toBe(JSON.stringify({ blueprint: 'engine_dirty', experimental: 'special_engine_overloaded', grade: 5, rolls: [1,1,1,1,4] }));
 click('pin-goal'); await settle(); click('close-flow'); await settle(); expect(actionBody('pin_goal')).toBe('{}'); expect(actionBody('close_flow')).toBe('{}');
});
test('flow renders material statuses per grade experimental totals readiness and locked engineer', () => {
 const flow = structuredClone(fixture['engineering.flow']); flow.lines[0] = { ...flow.lines[0], state: 'missing', missing: 2 }; flow.cost.replaces_blueprint = true; live.topics['engineering.flow'] = flow;
 showFlow(); expect(target.textContent).toContain('Falta 2'); expect(target.textContent).toContain('Cubierto con intercambio'); expect(target.textContent).toContain('Se sustituye'); expect(target.textContent).toContain('47 % listo'); expect(target.textContent).toContain('Sin verificar'); expect(target.textContent).toContain('Professor Palin'); expect(target.textContent).toContain('G5'); expect(target.querySelector('[data-testid="cost-experimental"]')?.textContent).toContain('1'); expect(target.querySelector('[data-testid="flow-next-step"]')?.textContent).not.toContain('unlock-professor-palin');
});
test('flow defaults from config and resets only on slot changes', async () => {
 const flow = { ...structuredClone(fixture['engineering.flow']), request: null, status: 'idle' }; live.topics['engineering.flow'] = flow;
 vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ engineering_rolls: [2,3,4,5,6] })))); showFlow(); await settle();
 expect((target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement).value).toBe('6'); change('rolls-5', '9'); change('target-grade', '3');
 live.topics['engineering.flow'] = { ...flow, as_of: 'later' }; flushSync(); expect((target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement).value).toBe('9');
 live.topics['engineering.flow'] = { ...flow, slot: 'Other' }; flushSync(); expect((target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement).value).toBe('6'); expect((target.querySelector('[data-testid="target-grade"]') as HTMLSelectElement).value).toBe('5');
});
test('late config never overwrites an edited draft and rejects invalid rolls', async () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: null };
 let finish!: (response: Response) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; }))); showFlow(); change('rolls-1', '7'); finish(new Response(JSON.stringify({ engineering_rolls: [2,2,2,2,2] }))); await settle(); expect((target.querySelector('[data-testid="rolls-1"]') as HTMLInputElement).value).toBe('7'); change('rolls-1', '11'); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(true);
});
test('flow supports null slot no data missing grades pending names and action errors', async () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], slot: null, status: 'idle' }; showFlow(); expect(target.querySelector('[data-testid="preview"]')).toBeNull();
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], status: 'no_data', cost: null }; flushSync(); expect(target.textContent).toContain('Sin datos');
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], slot: 'Other', options: { blueprints: ['unknown'], experimentals: ['unknown'] }, request: null, cost: { ...fixture['engineering.flow'].cost, missing_grades: [4] } }; flushSync(); expect(target.textContent).toContain('Traducción pendiente');
 vi.stubGlobal('fetch', vi.fn(async () => { throw new Error('offline'); })); click('preview'); await settle(); expect(target.querySelector('[role="alert"]')?.textContent).toContain('No se pudo');
});

test('flow preview uses edited grade rolls and null experimental', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}'))); showFlow(); change('experimental', ''); change('target-grade', '3'); change('rolls-3', '7'); click('preview'); await settle(); expect(actionBody('preview')).toBe(JSON.stringify({ blueprint: 'engine_dirty', experimental: null, grade: 3, rolls: [1,1,7,1,4] }));
});
test('commander ship and close changes reset flow draft but inventory does not', () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: null }; showFlow(); change('rolls-1', '8'); live.topics['engineering.inventory'] = { ...fixture['engineering.inventory'], as_of: 'later' }; flushSync(); expect((target.querySelector('[data-testid="rolls-1"]') as HTMLInputElement).value).toBe('8');
 live.topics['core.cmdr'] = { name: 'New' }; flushSync(); expect((target.querySelector('[data-testid="rolls-1"]') as HTMLInputElement).value).toBe('1'); change('rolls-1', '8'); live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 8 }; flushSync(); expect((target.querySelector('[data-testid="rolls-1"]') as HTMLInputElement).value).toBe('1');
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], slot: null }; flushSync(); live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: null }; flushSync(); expect((target.querySelector('[data-testid="rolls-1"]') as HTMLInputElement).value).toBe('1');
});
test('pending preview disables duplicate posts and obsolete failures cannot cross modules', async () => {
 showFlow(); let reject!: (error: Error) => void;
 vi.stubGlobal('fetch', vi.fn(() => new Promise((_resolve, fail) => { reject = fail; }))); click('preview'); click('preview'); expect(fetch).toHaveBeenCalledTimes(1);
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], slot: 'Other' }; flushSync(); reject(new Error('offline')); await settle(); expect(target.querySelector('[role="alert"]')).toBeNull(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(false);
});
test('edited draft cannot pin old preview while awaiting matching websocket request', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('{}'))); showFlow(); change('rolls-5', '9'); expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull(); expect(target.querySelector('[data-material="carbon"]')).toBeNull(); click('preview'); await settle(); expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(true);
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,9] } }; flushSync(); expect((target.querySelector('[data-testid="pin-goal"]') as HTMLButtonElement).disabled).toBe(false); click('pin-goal'); await settle(); expect(actionBody('pin_goal')).toBe('{}');
});
test('typing rolls protects draft from config before blur', async () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: null }; let finish!: (response: Response) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; }))); showFlow(); const input = target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement; input.value = '8'; input.dispatchEvent(new Event('input', { bubbles: true })); flushSync(); finish(new Response(JSON.stringify({ engineering_rolls: [2,2,2,2,2] }))); await settle(); expect(input.value).toBe('8');
});
test('ship selection waits for preview request and flow waits for selected slot confirmation', async () => {
 showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 let finish!: (response: Response) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; })));
 try {
  click('preview'); const select = shipTarget.querySelector('[data-testid="module-FrameShiftDrive"]') as HTMLButtonElement; expect(select.disabled).toBe(true); select.click(); expect(fetch).toHaveBeenCalledTimes(1);
  finish(new Response('{}')); await settle(); expect(select.disabled).toBe(false); select.click(); flushSync(); finish(new Response('{}')); await settle(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(true);
  live.topics['engineering.flow'] = { ...fixture['engineering.flow'], slot: 'FrameShiftDrive' }; flushSync(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(false);
 } finally { await unmount(ship); shipTarget.remove(); }
});
test('missing grade recipes remain visible with their concrete grades', () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], cost: { ...fixture['engineering.flow'].cost, missing_grades: [2,4] } }; showFlow(); expect(target.textContent).toContain('Sin datos: G2, G4');
});
test('invalid saved rolls use factory defaults and a missing request starts at grade five', async () => {
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: null }; vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ engineering_rolls: [0,1,1,1,11] })))); showFlow(); await settle(); expect((target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement).value).toBe('4'); expect((target.querySelector('[data-testid="target-grade"]') as HTMLSelectElement).value).toBe('5');
});
test('flow handles null experimental cost with no selected experimental or an already installed effect', async () => {
 for (const current of [null, { blueprint: 'engine_dirty', level: 3, quality: 1, experimental: 'special_engine_overloaded' }]) {
  live.topics['engineering.flow'] = { ...fixture['engineering.flow'], current, request: { ...fixture['engineering.flow'].request, experimental: current?.experimental ?? null }, cost: { ...fixture['engineering.flow'].cost, experimental: null } };
  showFlow();
  expect(target.querySelector('[data-testid="cost-experimental"]')).not.toBeNull();
  expect(target.querySelector('[data-material="carbon"]')).not.toBeNull();
  await unmount(mounted!); mounted = undefined;
 }
});
test('reselecting the current module cannot reset a confirmed preview', async () => {
 showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
 try {
  const selected = shipTarget.querySelector('[data-testid="module-MainEngines"]') as HTMLButtonElement;
  expect(selected.disabled).toBe(true); selected.click(); await settle();
  expect(fetch).not.toHaveBeenCalled();
  expect((target.querySelector('[data-testid="pin-goal"]') as HTMLButtonElement).disabled).toBe(false);
 } finally { await unmount(ship); shipTarget.remove(); }
});
test('acknowledged preview can be recalculated after another client updates the same slot', async () => {
 showFlow(); let finish!: (response: Response) => void;
 vi.stubGlobal('fetch', vi.fn(() => new Promise(resolve => { finish = resolve; })));
 change('rolls-5', '8'); click('preview'); finish(new Response('{}')); await settle();
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,8] } }; flushSync();
 expect((target.querySelector('[data-testid="pin-goal"]') as HTMLButtonElement).disabled).toBe(false);
 live.topics['engineering.flow'] = { ...fixture['engineering.flow'], request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,6] } }; flushSync();
 expect((target.querySelector('[data-testid="rolls-5"]') as HTMLInputElement).value).toBe('8');
 expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(false);
 click('preview'); finish(new Response('{}')); await settle(); expect(fetch).toHaveBeenCalledTimes(2);
});
test.each(['commander', 'ship'])('selection wait is invalidated after %s changes and ignores obsolete failure', async (changed) => {
 showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 const contextChange = () => {
  if (changed === 'commander') live.topics['core.cmdr'] = { name: 'Other' };
  else live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 9 };
  live.topics['engineering.flow'] = structuredClone(fixture['engineering.flow']); flushSync();
 };
 try {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('{}')));
  const select = shipTarget.querySelector('[data-testid="module-FrameShiftDrive"]') as HTMLButtonElement;
  select.click(); await settle(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(true);
  contextChange(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(false);
  // La primera selección no se confirma por WS; el segundo POST guarda esa espera anterior.
  select.click(); await settle();
  let reject!: (error: Error) => void; vi.stubGlobal('fetch', vi.fn(() => new Promise((_resolve, fail) => { reject = fail; })));
  select.click(); flushSync();
  if (changed === 'commander') live.topics['core.cmdr'] = { name: 'Third' };
  else live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 10 };
  flushSync(); expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(true);
  reject(new Error('obsolete')); await settle();
  expect((target.querySelector('[data-testid="preview"]') as HTMLButtonElement).disabled).toBe(false);
  expect((target.querySelector('[data-testid="close-flow"]') as HTMLButtonElement).disabled).toBe(false);
  expect((target.querySelector('[data-testid="pin-goal"]') as HTMLButtonElement).disabled).toBe(false);
  expect(shipTarget.querySelector('[role="alert"]')).toBeNull();
 } finally { await unmount(ship); shipTarget.remove(); }
});

import ShoppingPanel from '../src/modules/engineering/ShoppingPanel.svelte';
import TradesPanel from '../src/modules/engineering/TradesPanel.svelte';
import SourcesPanel from '../src/modules/engineering/SourcesPanel.svelte';
import EngineeringAlerts from '../src/modules/engineering/EngineeringAlerts.svelte';
function goalFetch() {
 vi.stubGlobal('fetch', vi.fn(async (url: string) => {
  if (url.startsWith('/api/actions/')) return new Response('{}');
  const query = new URL(url, 'http://localhost').searchParams;
  const section = query.get('section')!;
  return new Response(JSON.stringify({ ...fixture['engineering.pages'].goals, revision: Number(query.get('revision')), section, items: section === 'goals' ? fixture['engineering.pages'].goals.items : [{ key: 'iron', count: 7 }] }));
 }));
}
function panel(component: typeof ShoppingPanel) { mounted = mount(component, { target }); flushSync(); }
test('goal shows percent blueprint grade missing and completable engineer', async () => {
 goalFetch(); panel(ShoppingPanel); await settle();
 expect(target.textContent).toContain('87 %'); expect(target.textContent).toContain('G5'); expect(target.textContent).toContain('Felicity Farseer'); expect(target.textContent).toContain('Sin verificar');
 for (const section of ['needs', 'missing', 'surplus']) expect(target.querySelector(`[data-section="${section}"]`)?.textContent).toContain('7');
});
test('move and remove goals post exact json and localized errors', async () => {
 goalFetch(); live.topics['engineering.goals']={...fixture['engineering.goals'],total:2}; panel(ShoppingPanel); await settle(); click('goal-down-0'); await settle();
 expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.move_goal', expect.objectContaining({ body: JSON.stringify({ id: fixture['engineering.pages'].goals.items[0].id, to: 1 }) }));
 click('goal-remove-0'); await settle(); expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.unpin_goal', expect.objectContaining({ body: JSON.stringify({ id: fixture['engineering.pages'].goals.items[0].id }) }));
});
test('trade shows give receive names and nearby typed trader', async () => {
 goalFetch(); panel(TradesPanel); await settle(); expect(target.textContent).toContain('6'); expect(target.textContent).toContain('1'); expect(target.textContent).toContain('Eccentric Hyperspace Trajectories'); expect(target.textContent).toContain('Fake Port'); expect(target.textContent).toContain('Codificados');
});
test('sources show distance pad hints and applicable broker', () => {
 panel(SourcesPanel); expect(target.textContent).toContain('8,2 al'); expect(target.textContent).toContain('Fake Port'); expect(target.textContent).toContain('Synthetic Neighbour'); expect(target.textContent).toContain('L'); expect(target.textContent).toContain('Escaneo de datos'); expect(target.textContent).toContain('Buscando'); expect(fetch).not.toHaveBeenCalled();
});
test('source disabled nodata loading and unknown states remain readable', () => {
 live.topics['engineering.sources'] = { traders: { Raw: 'Disabled', Manufactured: 'NoData', Encoded: 'Future' }, brokers: {}, hints: { unknown: [{kind:'future',detail_key:'future-key'}] } }; panel(SourcesPanel);
 expect(target.querySelector('[data-trader="Raw"]')?.textContent).toContain('Sin datos'); expect(target.textContent).toContain('Servicios online desactivados'); expect(target.textContent).not.toContain('future-key'); expect(target.textContent).not.toContain('Future');
});
test('alerts resolve engineer names dismiss by id and localize failures', async () => {
 goalFetch(); panel(EngineeringAlerts); expect(target.textContent).toContain('Elvira Martuuk'); click('engineering-dismiss-0'); await settle();
 expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.dismiss_alert', expect.objectContaining({body:JSON.stringify({id:'unlocked:300160'})}));
 vi.mocked(fetch).mockRejectedValue(new Error('secret')); click('engineering-dismiss-0'); await settle(); expect(target.textContent).toContain('No se pudo completar la acción'); expect(target.textContent).not.toContain('secret');
});
test('unknown alert and material remain readable', () => {
 live.topics['engineering.alerts'] = { alerts: [{id:'x',kind:'future'}, {id:'y',kind:'material_at_max',key:'unknown',grade:0}] }; panel(EngineeringAlerts); expect(target.textContent).toContain('Sin datos'); expect(target.textContent).not.toContain('future');
});
test('shopping 409 waits for websocket revision without polling', async () => {
 vi.stubGlobal('fetch', vi.fn(async () => new Response('',{status:409}))); panel(ShoppingPanel); await settle(); expect(target.textContent).toContain('Esperando'); expect(fetch).toHaveBeenCalledTimes(4);
 await settle(); expect(fetch).toHaveBeenCalledTimes(4); live.topics['engineering.goals'] = {...fixture['engineering.goals'],revision: 22}; flushSync(); await settle(); expect(fetch).toHaveBeenCalledTimes(8);
});
test('shopping as_of does not restart requests and commander changes invalidate rows', async () => {
 goalFetch(); panel(ShoppingPanel); await settle(); vi.mocked(fetch).mockClear(); live.topics['engineering.goals'] = {...fixture['engineering.goals'],as_of:'2030-01-01'}; flushSync(); await settle(); expect(fetch).not.toHaveBeenCalled();
 vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {}))); live.topics['core.cmdr']={name:'Other'}; flushSync(); expect(target.textContent).not.toContain('87 %'); expect(fetch).toHaveBeenCalledTimes(4);
});
test('shopping cancels hidden disconnected and unmounted consumers', async () => {
 vi.stubGlobal('fetch',vi.fn(() => new Promise<Response>(() => {}))); panel(ShoppingPanel); const signal=vi.mocked(fetch).mock.calls[0][1]!.signal!;
 live.connected=false; flushSync(); expect(signal.aborted).toBe(true); live.connected=true; flushSync(); const second=vi.mocked(fetch).mock.calls[4][1]!.signal!;
 const hidden=vi.spyOn(document,'hidden','get').mockReturnValue(true); document.dispatchEvent(new Event('visibilitychange')); flushSync(); expect(second.aborted).toBe(true); hidden.mockReturnValue(false); document.dispatchEvent(new Event('visibilitychange')); flushSync(); hidden.mockRestore();
 const last=vi.mocked(fetch).mock.calls.at(-1)![1]!.signal!; await unmount(mounted!); mounted=undefined; expect(last.aborted).toBe(true);
});

test('missing known materials use approved catalog names and unknown keys show no data', async () => {
 live.topics['engineering.inventory']={by_category:{},caps:{},at_max:[],snapshot:true};
 goalFetch(); panel(ShoppingPanel); await settle(); expect(target.textContent).toContain('Datamined Wake Exceptions'); expect(target.textContent).toContain('Iron');
 live.topics['engineering.sources']={traders:{},brokers:{},hints:{new_material:[{kind:'data_scan',detail_key:null}]}};
 await unmount(mounted!); mounted=undefined; panel(SourcesPanel); expect(target.textContent).toContain('new_material'); expect(target.textContent).toContain('Sin datos');
});
test('shopping publishes all sections atomically across multiple pages', async () => {
 const goal=fixture['engineering.pages'].goals.items[0];
 let finish!: (response:Response)=>void;
 vi.stubGlobal('fetch',vi.fn(async (url:string) => {
  const q=new URL(url,'http://localhost').searchParams; const section=q.get('section')!; const cursor=Number(q.get('cursor'));
  if(section==='surplus') return new Promise<Response>(resolve=>{finish=resolve;});
  return new Response(JSON.stringify({revision: 21,section,cursor,total:section==='goals'?2:1,next_cursor:section==='goals' && cursor===0?1:null,items:section==='goals'?[{...goal,id:cursor?'second':goal.id}]:[{key:'iron',count:7}]}));
 }));
 panel(ShoppingPanel); await settle(); expect(target.textContent).not.toContain('87 %');
 finish(new Response(JSON.stringify({revision: 21,section:'surplus',cursor:0,next_cursor:null,total:1,items:[{key:'carbon',count:2}]}))); await settle();
 expect(target.querySelectorAll('[data-goal-id]')).toHaveLength(2); expect(target.querySelector('[data-section="surplus"]')?.textContent).toContain('2');
 click('goal-up-1'); await settle(); expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.move_goal',expect.objectContaining({body:JSON.stringify({id:'second',to:0})}));
});
test('late goals JSON from old revision cannot replace current commander rows', async () => {
 const goal=fixture['engineering.pages'].goals.items[0]; let finish!: (value:unknown)=>void;
 vi.stubGlobal('fetch',vi.fn().mockResolvedValueOnce({ok:true,status:200,json:()=>new Promise(resolve=>{finish=resolve;})}));
 panel(TradesPanel); await settle();
 goalFetch(); live.topics['engineering.goals']={...fixture['engineering.goals'],revision: 22}; live.topics['core.cmdr']={name:'Changed'}; flushSync(); await settle();
 finish({...fixture['engineering.pages'].goals,items:[{...goal,trades:[{...goal.trades[0],give:999}]}]}); await settle(); expect(target.textContent).not.toContain('999'); expect(target.textContent).toContain('6');
});
test('shopping action and page failures are localized', async () => {
 goalFetch(); panel(ShoppingPanel); await settle(); vi.mocked(fetch).mockRejectedValue(new Error('private')); click('goal-remove-0'); await settle(); expect(target.textContent).toContain('No se pudo completar la acción'); expect(target.textContent).not.toContain('private');
 live.topics['engineering.goals']={...fixture['engineering.goals'],revision: 22}; flushSync(); await settle(); expect(target.textContent).toContain('No se pudieron cargar los datos'); expect(target.textContent).not.toContain('87 %');
});
test('all known engineering alert kinds resolve readable names and goals', () => {
 live.topics['engineering.alerts']={alerts:[{id:'max',kind:'material_at_max',key:'iron',grade:1},{id:'invite',kind:'engineer_invited',engineer_id:300100},{id:'scarce',kind:'scarce_obtained',key:'iron'},{id:'goal',kind:'goal_completable',goal:fixture['engineering.pages'].goals.items[0].id,engineer:'Felicity Farseer'}]};
 panel(EngineeringAlerts); expect(target.textContent).toContain('Hierro al máximo'); expect(target.textContent).toContain('Invitación nueva de Felicity Farseer'); expect(target.textContent).toContain('Material escaso conseguido'); expect(target.textContent).toContain('G5'); expect(target.textContent).not.toContain('fsd_longrange');
});

test('late shopping and dismiss errors cannot cross commanders or release the HTTP lock early', async () => {
 for (const component of [ShoppingPanel,EngineeringAlerts]) {
  goalFetch(); panel(component); await settle();
  let fail!: (error:Error)=>void; const pages=fetch; vi.stubGlobal('fetch',vi.fn((url:string,init?:RequestInit)=>url.startsWith('/api/actions/') ? new Promise<Response>((_,reject)=>{fail=reject;}) : pages(url,init)));
  const id=component===ShoppingPanel ? 'goal-remove-0' : 'engineering-dismiss-0'; click(id);
  live.topics['core.cmdr']={name:'Another context'}; flushSync(); await settle();
  expect((target.querySelector(`[data-testid="${id}"]`) as HTMLButtonElement | null)?.disabled ?? true).toBe(true);
  fail(new Error('old commander')); await settle(); expect(target.textContent).not.toContain('No se pudo completar la acción');
  await unmount(mounted!); mounted=undefined; live.topics['core.cmdr']={name:'Synthetic'}; flushSync();
 }
});
