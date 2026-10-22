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
import { resetEngineerNames } from '../src/modules/engineering/engineerNames.svelte';
let target: HTMLDivElement;
let mounted: ReturnType<typeof mount> | undefined;
beforeEach(() => {
 engineeringActions.pending = false; engineeringActions.selectedSlot = null; resetEngineerNames();
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
type Rest = { items: unknown[] } & Record<string, unknown>;
const restPages = fixture['engineering.pages'] as unknown as Record<string, Rest>;
interface RestOptions { config?: unknown | (() => Promise<Response>); action?: (url: string, init?: RequestInit) => Response | Promise<Response>; override?: (resource: string, section: string, query: URLSearchParams) => Response | Promise<Response> | undefined }
/** Servidor falso de los recursos REST de ingeniería, las acciones y la configuración. */
function restFetch(options: RestOptions = {}) {
 vi.stubGlobal('fetch', vi.fn(async (url: string, init?: RequestInit) => {
  if (url.startsWith('/api/actions/')) return options.action ? options.action(url, init) : new Response('{}');
  if (url === '/api/config') return typeof options.config === 'function' ? options.config() : new Response(JSON.stringify(options.config ?? {}));
  const parsed = new URL(url, 'http://localhost'); const resource = parsed.pathname.split('/').pop()!;
  const section = parsed.searchParams.get('section') ?? (resource === 'flow' ? 'lines' : 'engineers');
  const custom = options.override?.(resource, section, parsed.searchParams); if (custom) return custom;
  if (resource === 'engineers' && section === 'capabilities') {
   const rev = Number(parsed.searchParams.get('revision')) || 21;
   const engId = Number(parsed.searchParams.get('engineer_id'));
   if (engId === 300100) {
    return json(page('capabilities', [
     { engineer_id: 300100, module_type: 'engine', blueprint: 'engine_dirty', grade: 1 },
     { engineer_id: 300100, module_type: 'engine', blueprint: 'engine_dirty', grade: 2 },
     { engineer_id: 300100, module_type: 'engine', blueprint: 'engine_dirty', grade: 3 },
    ], rev));
   }
   if (engId === 300160) {
    return json(page('capabilities', [
     { engineer_id: 300160, module_type: 'engine', blueprint: 'engine_dirty', grade: 1 },
     { engineer_id: 300160, module_type: 'engine', blueprint: 'engine_dirty', grade: 2 },
    ], rev));
   }
  }
  const base = restPages[`${resource}_${section}`] ?? restPages[resource];
  return new Response(JSON.stringify({ ...base, revision: Number(parsed.searchParams.get('revision')), section }));
 }));
}
function restCalls(resource: string) { return vi.mocked(fetch).mock.calls.filter(([url]) => String(url).startsWith(`/api/engineering/${resource}?`)); }
function actionCalls(name: string) { return vi.mocked(fetch).mock.calls.filter(([url]) => url === `/api/actions/engineering.${name}`); }
function json(body: unknown) { return new Response(JSON.stringify(body)); }
function page(section: string, items: unknown[], revision = 21) { return { revision, section, cursor: 0, next_cursor: null, total: items.length, items }; }
async function showEngineers(options: RestOptions = {}) {
 restFetch(options);
 mounted = mount(EngineersPanel, { target });
 flushSync(); await settle();
}
function selectEngineer(next_step: EngineerNextStep = 'Done') {
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300100, next_step };
}
const capabilityOf = (blueprint: string, grades: number[]) => (resource: string, section: string) => resource === 'engineers' && section === 'capabilities'
 ? json(page('capabilities', grades.map(grade => ({ engineer_id: 300100, module_type: 'engine', blueprint, grade })))) : undefined;
test('engineer topic is a summary and rows come from the paginated resource', async () => {
 expect(Object.keys(fixture['engineering.engineers']).sort()).toEqual(['as_of', 'next_step', 'revision', 'selected_id', 'total']);
 await showEngineers();
 expect(restCalls('engineers').map(([url]) => String(url))).toEqual(['/api/engineering/engineers?revision=21&cursor=0&section=engineers']);
});
test('engineer row shows status rank progress system distance and verification', async () => {
 await showEngineers();
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
 await showEngineers();
 (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 await settle();
 expect(fetch).toHaveBeenCalledWith('/api/actions/engineering.select_engineer', {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ id: 300100 }),
 });
});
test('next step contribution shows have and need', async () => {
 selectEngineer({ Contribute: { item: 'iron', kind: 'material', have: 4, need: 10 } });
 await showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('4 / 10');
 expect(target.textContent).toContain('Hierro');
});
test('on foot engineers in their own section', async () => {
 await showEngineers();
 expect(target.querySelector('[data-testid="engineers-on-foot"]')?.textContent).toContain('Domino Green');
 expect(target.querySelector('[data-testid="engineers-ship"]')?.textContent).not.toContain('Domino Green');
});
test('selected engineer capabilities show module, official blueprint name and exact grades', async () => {
 selectEngineer(); await showEngineers();
 const text = target.querySelector('[data-testid="engineer-blueprints"]')?.textContent ?? '';
 expect(text).toContain('Impulsores'); expect(text).toContain('Modificación de Motor Sucio'); expect(text).toContain('Impulsores · Modificación de Motor Sucio: G1, G2, G3');
 expect(text).not.toContain('Modificación de Motor Sucio: G1, G2, G3, G4'); expect(text).not.toContain('Traducción pendiente');
 expect(restCalls('engineers').map(([url]) => new URL(String(url), 'http://x').searchParams.get('section')).sort()).toEqual(['capabilities', 'engineers']);
});
test('capabilities are not requested until an engineer is selected and only that engineer is shown', async () => {
 await showEngineers(); expect(restCalls('engineers')).toHaveLength(1);
 selectEngineer(); flushSync(); await settle(); expect(restCalls('engineers')).toHaveLength(2);
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('G1, G2, G3');
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300160 }; flushSync();
 await settle(); expect(restCalls('engineers')).toHaveLength(3);
 const text = target.querySelector('[data-testid="engineer-blueprints"]')?.textContent ?? '';
 expect(text).toContain('Modificación de Motor Sucio: G1, G2'); expect(text).not.toContain('Modificación de Motor Sucio: G1, G2, G3');
});
test('a new revision discards old rows and reloads the pages', async () => {
 selectEngineer(); await showEngineers();
 let finish!: (response: Response) => void;
 restFetch({ override: () => new Promise<Response>(resolve => { finish = resolve; }) as unknown as Response });
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300100, next_step: 'Done', revision: 22 }; flushSync();
 expect(target.querySelector('[data-engineer-id="300100"]')).toBeNull(); expect(target.textContent).toContain('Cargando');
 expect(restCalls('engineers').map(([url]) => String(url)).every(url => url.includes('revision=22'))).toBe(true);
 finish(json(page('engineers', [], 22)));
});
test('engineer rows of two revisions are never mixed', async () => {
 selectEngineer(); await showEngineers();
 restFetch({ override: (resource, section, query) => resource === 'engineers' && section === 'engineers' && query.get('revision') === '22' ? json(page('engineers', [{ id: 1, name: 'Only Rev 22', on_foot: false, status: 'Known', rank: null, rank_progress: null, system: null, distance_ly: null, verified: true }], 22)) : undefined });
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], revision: 22 }; flushSync(); await settle();
 expect(target.textContent).toContain('Only Rev 22'); expect(target.textContent).not.toContain('Felicity Farseer');
});
test('engineer pages failure and stale are recoverable states', async () => {
 await showEngineers({ override: () => new Response('', { status: 500 }) }); expect(target.textContent).toContain('No se pudieron cargar los datos');
 await unmount(mounted!); mounted = undefined;
 await showEngineers({ override: () => new Response('', { status: 409 }) }); expect(target.textContent).toContain('Esperando');
 expect(restCalls('engineers')).toHaveLength(1);
});
test('engineer pages are cancelled on unmount and when the document is hidden', async () => {
 restFetch({ override: () => new Promise<Response>(() => {}) as unknown as Response });
 mounted = mount(EngineersPanel, { target }); flushSync();
 const signal = vi.mocked(fetch).mock.calls[0][1]!.signal!;
 const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true); document.dispatchEvent(new Event('visibilitychange')); flushSync();
 expect(signal.aborted).toBe(true); hidden.mockRestore(); document.dispatchEvent(new Event('visibilitychange')); flushSync();
 const second = vi.mocked(fetch).mock.calls.at(-1)![1]!.signal!; await unmount(mounted!); mounted = undefined; expect(second.aborted).toBe(true);
});
test('all next step variants are rendered without raw Fluent keys', async () => {
 selectEngineer({ Requirement: { key: 'unlock-felicity-farseer' } }); await showEngineers();
 expect(target.textContent).toContain('Cumple el requisito');
 selectEngineer({ Invite: { key: 'unlock-invite-felicity-farseer' } }); flushSync();
 expect(target.textContent).toContain('Consigue la invitación');
 selectEngineer('Done'); flushSync();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Desbloqueado');
});
test('unknown status null rank progress and distance show localized no data', async () => {
 const row = { ...restPages.engineers_engineers.items[0] as object, status: 'FutureStatus', rank: null, rank_progress: null, distance_ly: null };
 await showEngineers({ override: (resource, section) => resource === 'engineers' && section === 'engineers' ? json(page('engineers', [row])) : undefined });
 const element = target.querySelector('[data-engineer-id="300100"]')!;
 expect(element.textContent).toContain('Desconocido'); expect(element.textContent).toContain('Sin datos');
 expect(element.textContent).not.toContain('FutureStatus'); expect(element.textContent).not.toContain('null');
});
test('unknown blueprint uses English fallback key and marks translation pending', async () => {
 expect(blueprintLabel('unknown_blueprint', 'New Drive Tuning')).toEqual({ name: 'New Drive Tuning', pending: true });
 selectEngineer('Done'); await showEngineers({ override: capabilityOf('unknown_blueprint', [2]) });
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('Traducción pendiente');
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('G2');
});
test('engineer selection failure shows localized error', async () => {
 await showEngineers({ action: () => new Response('{}', { status: 500 }) }); (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 await settle(); expect(target.textContent).toContain('No se pudo seleccionar el ingeniero');
});
test('missing engineers topic renders no data', async () => {
 delete live.topics['engineering.engineers']; await showEngineers();
 expect(target.textContent).toContain('Sin datos'); expect(fetch).not.toHaveBeenCalled();
});
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { translationPending } from '../src/lib/i18n';
test('pending Fluent attribute is read from a real compiled resource', async () => {
 const bundle = new FluentBundle('es');
 bundle.addResource(new FluentResource('bp-synthetic = English Drive Tuning\n    .pending = 1'));
 const original = FluentBundle.prototype.getMessage;
 const spy = vi.spyOn(FluentBundle.prototype, 'getMessage').mockImplementation(function (this: FluentBundle, key: string) {
  return key === 'bp-synthetic' ? original.call(bundle, key) : original.call(this, key);
 });
 try {
  expect(blueprintLabel('synthetic')).toEqual({ name: 'English Drive Tuning', pending: true });
  expect(translationPending('bp-engine_dirty')).toBe(false);
  selectEngineer('Done'); await showEngineers({ override: capabilityOf('synthetic', [1]) });
  expect(target.textContent).toContain('English Drive Tuning');
  expect(target.textContent).toContain('Traducción pendiente');
 } finally { spy.mockRestore(); }
});
test('obsolete engineer selection failure cannot overwrite a newer successful selection', async () => {
 let rejectFirst!: (error: Error) => void; let calls = 0;
 await showEngineers({ action: () => ++calls === 1 ? new Promise<Response>((_, reject) => { rejectFirst = reject; }) : new Response('{}') });
 (target.querySelector('[data-engineer-id="300100"] button') as HTMLButtonElement).click();
 (target.querySelector('[data-engineer-id="300160"] button') as HTMLButtonElement).click();
 await settle(); rejectFirst(new Error('Unavailable')); await settle();
 expect(target.textContent).not.toContain('No se pudo seleccionar el ingeniero');
});
test('zero commodity contribution has readable catalog name and pending translation', async () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 0, need: 3 } });
 await showEngineers();
 const step = target.querySelector('[data-testid="engineer-next-step"]')!;
 expect(step.textContent).toContain('Soontill Relics');
 expect(step.textContent).toContain('0 / 3');
 expect(step.textContent).toContain('Traducción pendiente');
 expect(step.textContent).not.toContain('soontillrelics');
});
test('commodity contribution uses core cargo journal name and updates reactively', async () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 2, need: 3 } });
 live.topics['core.ship'] = { cargo: [{ Name: 'soontillrelics', Name_Localised: 'Nombre del diario', Count: 2 }] };
 await showEngineers();
 const step = target.querySelector('[data-testid="engineer-next-step"]')!;
 expect(step.textContent).toContain('Nombre del diario'); expect(step.textContent).toContain('2 / 3');
 expect(step.textContent).not.toContain('Traducción pendiente');
 live.topics['core.ship'] = { cargo: [] }; setLocale('en'); flushSync();
 expect(step.textContent).toContain('Soontill Relics'); expect(step.textContent).not.toContain('Translation pending');
});
test('commodity contribution uses trade cargo localized name when available', async () => {
 selectEngineer({ Contribute: { item: 'soontillrelics', kind: 'commodity', have: 2, need: 3 } });
 live.topics['trade.cargo'] = { lines: [{ commodity: 'soontillrelics', name_localised: 'Mercancía del diario', tons: 2 }] };
 await showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Mercancía del diario');
});
test('credit contribution uses localized credits and backend counts', async () => {
 selectEngineer({ Contribute: { item: 'credits', kind: 'credits', have: 4, need: 10 } });
 await showEngineers();
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Créditos');
 expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('4 / 10');
});
function showShip() { restFetch(); mounted = mount(ShipPanel, { target }); flushSync(); }
async function showFlow(options: RestOptions = {}) { restFetch(options); mounted = mount(EngineerFlow, { target }); flushSync(); await settle(); }
function change(id: string, value: string) { const input = target.querySelector(`[data-testid="${id}"]`) as HTMLInputElement; input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); input.dispatchEvent(new Event('change', { bubbles: true })); flushSync(); }
function actionBody(name: string) { return vi.mocked(fetch).mock.calls.find(([url]) => url === `/api/actions/engineering.${name}`)?.[1]?.body; }
const flowTopic = () => structuredClone(fixture['engineering.flow']) as Record<string, any>;
const input = (id: string) => target.querySelector(`[data-testid="${id}"]`) as HTMLInputElement | HTMLSelectElement;
const button = (id: string) => target.querySelector(`[data-testid="${id}"]`) as HTMLButtonElement;
const blueprintRows = (rows: unknown[]) => (resource: string, section: string) => resource === 'flow' && section === 'blueprints' ? json(page('blueprints', rows)) : undefined;
test('flow topic is a summary and every heavy section comes from the paginated resource', async () => {
 const topicKeys = Object.keys(fixture['engineering.flow']);
 for (const heavy of ['lines', 'cost', 'options']) expect(topicKeys).not.toContain(heavy);
 await showFlow();
 const sections = restCalls('flow').map(([url]) => new URL(String(url), 'http://x').searchParams.get('section')).sort();
 expect(sections).toEqual(['blueprints', 'costs', 'experimentals', 'grades', 'lines']);
 expect(target.querySelector('[data-material="pharmaceuticalisolators"]')).not.toBeNull();
});
test('ship shows stock and official current engineering including unsupported modules', async () => {
 const ship = structuredClone(fixture['engineering.ship']); ship.modules.push({ ...ship.modules[1], slot: 'Unsupported', engineerable: false }); live.topics['engineering.ship'] = ship;
 showShip(); expect(target.textContent).toContain('De serie'); expect(target.textContent).toContain('Aumento de Alcance de MDD'); expect(target.textContent).toContain('100 %');
 expect(target.querySelector('[data-slot="Unsupported"] button')).toBeNull();
 expect(target.querySelector('[data-slot="Unsupported"]')?.textContent).toContain('Aumento de Alcance de MDD');
 expect(target.querySelector('[data-slot="Unsupported"]')?.textContent).toContain('G4');
 click('module-FrameShiftDrive'); await settle(); expect(actionBody('select_module')).toBe(JSON.stringify({ slot: 'FrameShiftDrive' }));
});
test('preview posts exact request and pin and close post empty objects', async () => {
 await showFlow(); click('preview'); await settle();
 expect(actionBody('preview')).toBe(JSON.stringify({ blueprint: 'engine_dirty', experimental: 'special_engine_overloaded', grade: 5, rolls: [1,1,1,1,4] }));
 click('pin-goal'); await settle(); click('close-flow'); await settle(); expect(actionBody('pin_goal')).toBe('{}'); expect(actionBody('close_flow')).toBe('{}');
});
test('flow renders material statuses per grade experimental totals readiness and locked engineer', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), replaces_blueprint: true };
 const lines = (restPages.flow_lines.items as { key: string }[]).map(row => row.key === 'legacyfirmware' ? { ...row, state: 'missing', missing: 2 } : row);
 await showFlow({ override: (resource, section) => resource === 'flow' && section === 'lines' ? json(page('lines', lines)) : undefined });
 expect(target.textContent).toContain('Falta 2'); expect(target.textContent).toContain('Cubierto con intercambio'); expect(target.textContent).toContain('Se sustituye');
 expect(target.textContent).toContain('Sin verificar'); expect(target.textContent).toContain('Professor Palin'); expect(target.textContent).toContain('G5');
 expect(target.querySelector('[data-testid="cost-experimental"]')?.textContent).toContain('1');
 expect(target.querySelector('[data-testid="cost-grade-5"]')?.textContent).toContain('4');
 expect(target.querySelector('[data-testid="flow-next-step"]')?.textContent).not.toContain('unlock-professor-palin');
});
test('ready percent states that it refers to materials', async () => {
 await showFlow(); expect(target.querySelector('[data-testid="materials-percent"]')?.textContent).toBe('Materiales: 30 % listo');
 setLocale('en'); flushSync(); expect(target.querySelector('[data-testid="materials-percent"]')?.textContent).toBe('Materials: 30 % ready');
});
test('flow defaults from config and resets only on slot changes', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null, status: 'idle' };
 await showFlow({ config: { engineering_rolls: [2,3,4,5,6] } });
 expect((input('rolls-5') as HTMLInputElement).value).toBe('6'); change('rolls-5', '9');
 live.topics['engineering.flow'] = { ...flowTopic(), request: null, status: 'idle', as_of: 'later' }; flushSync(); await settle(); expect((input('rolls-5') as HTMLInputElement).value).toBe('9');
 change('target-grade', '3');
 live.topics['engineering.flow'] = { ...flowTopic(), request: null, status: 'idle', slot: 'Other' }; flushSync(); await settle();
 expect((input('rolls-5') as HTMLInputElement).value).toBe('6'); expect((input('target-grade') as HTMLSelectElement).value).toBe('5');
});
test('late config never overwrites an edited draft and rejects invalid rolls', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null };
 let finish!: (response: Response) => void;
 await showFlow({ config: () => new Promise<Response>(resolve => { finish = resolve; }) });
 change('rolls-1', '7'); finish(json({ engineering_rolls: [2,2,2,2,2] })); await settle();
 expect((input('rolls-1') as HTMLInputElement).value).toBe('7'); change('rolls-1', '11'); expect(button('preview').disabled).toBe(true);
});
test('flow without slot offers nothing and requests no pages', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), slot: null, status: 'idle' }; await showFlow();
 expect(target.querySelector('[data-testid="preview"]')).toBeNull(); expect(restCalls('flow')).toHaveLength(0);
});
test('flow visibility follows the open slot, not the calculation status', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), status: 'idle', request: null }; await showFlow();
 expect(target.querySelector('[data-testid="preview"]')).not.toBeNull();
});
test('no data without blockers is plain no data', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), status: 'no_data', blockers: [] }; await showFlow();
 expect(target.textContent).toContain('Sin datos'); expect(target.querySelector('[data-testid="flow-blockers"]')).toBeNull();
});
test('blocked flow shows the localized reason instead of plain no data', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), status: 'no_data', blockers: ['ambiguous_recipe', 'base_required'] }; await showFlow();
 const reasons = target.querySelector('[data-testid="flow-blockers"]')!.textContent!;
 expect(reasons).toContain('Hay varias recetas posibles'); expect(reasons).toContain('base previa'); expect(reasons).not.toContain('ambiguous_recipe');
 expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull();
});
test('missing grade recipes remain visible with their concrete grades', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), blockers: [{ missing_recipe: { grade: 2 } }, { missing_recipe: { grade: 4 } }] }; await showFlow();
 const reasons = target.querySelector('[data-testid="flow-blockers"]')!.textContent!; expect(reasons).toContain('Falta la receta de G2'); expect(reasons).toContain('Falta la receta de G4');
 expect(target.querySelector('[data-material="legacyfirmware"]')).not.toBeNull();
});
test('materials ready with unknown Merc Coins balance is not completable and shows the coins separately', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), materials_ready: true, percent: 100, merc_coins: { need: 30, have: null, missing: null } };
 const grades = (restPages.flow_grades.items as { grade: number }[]).map(row => row.grade === 5 ? { ...row, merc_coins: 30 } : row);
 await showFlow({ override: (resource, section) => resource === 'flow' && section === 'grades' ? json(page('grades', grades)) : undefined });
 expect(target.querySelector('[data-testid="merc-coins"]')?.textContent).toContain('Merc Coins necesarias: 30'); expect(target.querySelector('[data-testid="merc-coins"]')?.textContent).toContain('saldo sin datos');
 expect(target.querySelector('[data-testid="merc-not-confirmed"]')?.textContent).toContain('no se puede marcar como completable');
 expect(target.querySelector('[data-testid="cost-grade-5"]')?.textContent).toContain('Merc Coins: 30');
 expect(target.textContent).not.toContain('Ya puedes completarlo'); expect(target.textContent).not.toContain('saldo: 0');
});
test('known Merc Coins balance is shown and no warning is raised', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), materials_ready: true, merc_coins: { need: 30, have: 50, missing: 0 } }; await showFlow();
 expect(target.querySelector('[data-testid="merc-coins"]')?.textContent).toContain('saldo: 50'); expect(target.querySelector('[data-testid="merc-not-confirmed"]')).toBeNull();
});
test('experimental Merc Coins cost is listed apart from material costs', async () => {
 const costs = [...restPages.flow_costs.items, { grade: null, kind: 'merc_coins', count: 12 }];
 await showFlow({ override: (resource, section) => resource === 'flow' && section === 'costs' ? json(page('costs', costs)) : undefined });
 expect(target.querySelector('[data-testid="cost-experimental"]')?.textContent).toContain('Merc Coins: 12');
});
test('base-only recipes do not offer grades below their first grade', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null };
 await showFlow({ override: blueprintRows([{ key: 'engine_dirty', available: true, reason: null, grades: [4, 5] }]) });
 expect(Array.from((input('target-grade') as HTMLSelectElement).options).map(option => option.textContent)).toEqual(['G4', 'G5']);
 expect(target.querySelector('[data-testid="rolls-1"]')).toBeNull(); expect(target.querySelector('[data-testid="rolls-3"]')).toBeNull();
 expect(target.querySelector('[data-testid="rolls-4"]')).not.toBeNull(); expect((input('target-grade') as HTMLSelectElement).value).toBe('5');
 click('preview'); await settle(); expect(JSON.parse(String(actionBody('preview'))).grade).toBe(5);
});
test('without grade five the highest applicable grade is the default', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null };
 await showFlow({ override: blueprintRows([{ key: 'engine_dirty', available: true, reason: null, grades: [2, 3, 4] }]) });
 expect((input('target-grade') as HTMLSelectElement).value).toBe('4');
});
test.each([['missing_base_mapping', 'No se sabe qué base previa'], ['ambiguous_recipe', 'varias recetas posibles']])('unavailable blueprint (%s) blocks preview and states the reason', async (reason, text) => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null };
 await showFlow({ override: blueprintRows([{ key: 'engine_dirty', available: false, reason, grades: [] }]) });
 const option = (input('blueprint') as HTMLSelectElement).options[0];
 expect(option.disabled).toBe(true); expect(option.textContent).toContain(text);
 expect(button('preview').disabled).toBe(true); expect(target.querySelector('[data-testid="blueprint-blocked"]')?.textContent).toContain(text);
 click('preview'); await settle(); expect(actionCalls('preview')).toHaveLength(0);
});
test('a blueprint that cannot be applied is never the default choice', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null };
 await showFlow({ override: blueprintRows([{ key: 'broken', available: false, reason: 'base_required', grades: [] }, { key: 'engine_dirty', available: true, reason: null, grades: [1, 5] }]) });
 expect((input('blueprint') as HTMLSelectElement).value).toBe('engine_dirty');
});
test('flow pages of two revisions never mix and the old slot rows disappear at once', async () => {
 await showFlow(); expect(target.querySelector('[data-material="legacyfirmware"]')).not.toBeNull();
 const next = [{ ...(restPages.flow_lines.items[0] as object), key: 'newmaterial', name_en: 'New Material' }];
 restFetch({ override: (resource, section, query) => resource === 'flow' && section === 'lines' && query.get('revision') === '22' ? json(page('lines', next, 22)) : undefined });
 live.topics['engineering.flow'] = { ...flowTopic(), revision: 22 }; flushSync();
 expect(target.querySelector('[data-material="legacyfirmware"]')).toBeNull(); await settle();
 expect(target.querySelector('[data-material="newmaterial"]')).not.toBeNull(); expect(target.querySelector('[data-material="legacyfirmware"]')).toBeNull();
});
test('a slot change discards the previous rows before the new ones arrive', async () => {
 await showFlow(); restFetch({ override: () => new Promise<Response>(() => {}) as unknown as Response });
 live.topics['engineering.flow'] = { ...flowTopic(), slot: 'Other' }; flushSync();
 expect(target.querySelector('[data-material="legacyfirmware"]')).toBeNull(); expect(target.textContent).toContain('Cargando');
});
test('flow as_of-only updates do not restart the pages and unmount cancels them', async () => {
 await showFlow(); vi.mocked(fetch).mockClear();
 live.topics['engineering.flow'] = { ...flowTopic(), as_of: 'later' }; flushSync(); await settle(); expect(fetch).not.toHaveBeenCalled();
 restFetch({ override: () => new Promise<Response>(() => {}) as unknown as Response });
 live.topics['engineering.flow'] = { ...flowTopic(), revision: 23 }; flushSync();
 const signals = restCalls('flow').map(call => call[1]!.signal!); expect(signals).toHaveLength(5);
 await unmount(mounted!); mounted = undefined; expect(signals.every(signal => signal.aborted)).toBe(true);
});
test('flow pages failure and 409 are recoverable states without polling', async () => {
 await showFlow({ override: () => new Response('', { status: 500 }) }); expect(target.textContent).toContain('No se pudieron cargar los datos'); expect(restCalls('flow')).toHaveLength(5);
 await unmount(mounted!); mounted = undefined;
 await showFlow({ override: () => new Response('', { status: 409 }) }); expect(target.textContent).toContain('Esperando los datos actualizados'); await settle(); expect(restCalls('flow')).toHaveLength(5);
 restFetch(); live.topics['engineering.flow'] = { ...flowTopic(), revision: 22 }; flushSync(); await settle(); expect(target.querySelector('[data-material="legacyfirmware"]')).not.toBeNull();
});
test('flow supports pending names and action errors', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), slot: 'Other', request: null };
 await showFlow({ override: (resource, section) => resource === 'flow' && section === 'blueprints' ? json(page('blueprints', [{ key: 'unknown', available: true, reason: null, grades: [5] }])) : resource === 'flow' && section === 'experimentals' ? json(page('experimentals', [{ key: 'unknown' }])) : undefined });
 expect(target.textContent).toContain('Traducción pendiente');
 restFetch({ action: () => { throw new Error('offline'); } }); click('preview'); await settle(); expect(target.querySelector('[role="alert"]')?.textContent).toContain('No se pudo');
});
test('flow preview uses edited grade rolls and null experimental', async () => {
 await showFlow(); change('experimental', ''); change('target-grade', '3'); change('rolls-3', '7'); click('preview'); await settle(); expect(actionBody('preview')).toBe(JSON.stringify({ blueprint: 'engine_dirty', experimental: null, grade: 3, rolls: [1,1,7,1,4] }));
});
test('commander ship and close changes reset flow draft but inventory does not', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null }; await showFlow(); change('rolls-1', '8'); live.topics['engineering.inventory'] = { ...fixture['engineering.inventory'], as_of: 'later' }; flushSync(); expect((input('rolls-1') as HTMLInputElement).value).toBe('8');
 live.topics['core.cmdr'] = { name: 'New' }; flushSync(); await settle(); expect((input('rolls-1') as HTMLInputElement).value).toBe('1'); change('rolls-1', '8'); live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 8 }; flushSync(); await settle(); expect((input('rolls-1') as HTMLInputElement).value).toBe('1');
 live.topics['engineering.flow'] = { ...flowTopic(), slot: null }; flushSync(); live.topics['engineering.flow'] = { ...flowTopic(), request: null }; flushSync(); await settle(); expect((input('rolls-1') as HTMLInputElement).value).toBe('1');
});
test('pending preview disables duplicate posts and obsolete failures cannot cross modules', async () => {
 await showFlow(); let reject!: (error: Error) => void;
 restFetch({ action: () => new Promise<Response>((_resolve, fail) => { reject = fail; }) }); click('preview'); click('preview'); expect(actionCalls('preview')).toHaveLength(1);
 live.topics['engineering.flow'] = { ...flowTopic(), slot: 'Other' }; flushSync(); await settle(); reject(new Error('offline')); await settle(); expect(target.querySelector('[role="alert"]')).toBeNull(); expect(button('preview').disabled).toBe(false);
});
test('edited draft cannot pin old preview while awaiting matching websocket request', async () => {
 await showFlow(); change('rolls-5', '9'); expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull(); expect(target.querySelector('[data-material="legacyfirmware"]')).toBeNull(); click('preview'); await settle(); expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull(); expect(button('preview').disabled).toBe(true);
 live.topics['engineering.flow'] = { ...flowTopic(), request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,9] } }; flushSync(); await settle(); expect(button('pin-goal').disabled).toBe(false); click('pin-goal'); await settle(); expect(actionBody('pin_goal')).toBe('{}');
});
test('typing rolls protects draft from config before blur', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null }; let finish!: (response: Response) => void;
 await showFlow({ config: () => new Promise<Response>(resolve => { finish = resolve; }) }); const field = input('rolls-5') as HTMLInputElement; field.value = '8'; field.dispatchEvent(new Event('input', { bubbles: true })); flushSync(); finish(json({ engineering_rolls: [2,2,2,2,2] })); await settle(); expect(field.value).toBe('8');
});
test('ship selection waits for preview request and flow waits for selected slot confirmation', async () => {
 await showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 let finish!: (response: Response) => void; restFetch({ action: () => new Promise<Response>(resolve => { finish = resolve; }) });
 try {
  click('preview'); const select = shipTarget.querySelector('[data-testid="module-FrameShiftDrive"]') as HTMLButtonElement; expect(select.disabled).toBe(true); select.click(); expect(actionCalls('preview')).toHaveLength(1); expect(actionCalls('select_module')).toHaveLength(0);
  finish(new Response('{}')); await settle(); expect(select.disabled).toBe(false); select.click(); flushSync(); finish(new Response('{}')); await settle(); expect(button('preview').disabled).toBe(true);
  live.topics['engineering.flow'] = { ...flowTopic(), slot: 'FrameShiftDrive' }; flushSync(); await settle(); expect(button('preview').disabled).toBe(false);
 } finally { await unmount(ship); shipTarget.remove(); }
});
test('invalid saved rolls use factory defaults and a missing request starts at grade five', async () => {
 live.topics['engineering.flow'] = { ...flowTopic(), request: null }; await showFlow({ config: { engineering_rolls: [0,1,1,1,11] } }); expect((input('rolls-5') as HTMLInputElement).value).toBe('4'); expect((input('target-grade') as HTMLSelectElement).value).toBe('5');
});
test('flow handles no experimental rows with no selected experimental or an already installed effect', async () => {
 const costs = (restPages.flow_costs.items as { grade: number | null }[]).filter(row => row.grade !== null);
 for (const current of [null, { blueprint: 'engine_dirty', level: 3, quality: 1, experimental: 'special_engine_overloaded' }]) {
  live.topics['engineering.flow'] = { ...flowTopic(), current, request: { ...fixture['engineering.flow'].request, experimental: current?.experimental ?? null } };
  await showFlow({ override: (resource, section) => resource === 'flow' && section === 'costs' ? json(page('costs', costs)) : undefined });
  expect(target.querySelector('[data-testid="cost-experimental"]')).not.toBeNull();
  expect(target.querySelector('[data-material="legacyfirmware"]')).not.toBeNull();
  await unmount(mounted!); mounted = undefined;
 }
});
test('reselecting the current module cannot reset a confirmed preview', async () => {
 await showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 restFetch({ action: () => new Promise<Response>(() => {}) });
 try {
  const selected = shipTarget.querySelector('[data-testid="module-MainEngines"]') as HTMLButtonElement;
  expect(selected.disabled).toBe(true); selected.click(); await settle();
  expect(actionCalls('select_module')).toHaveLength(0);
  expect(button('pin-goal').disabled).toBe(false);
 } finally { await unmount(ship); shipTarget.remove(); }
});
test('acknowledged preview can be recalculated after another client updates the same slot', async () => {
 await showFlow(); let finish!: (response: Response) => void;
 restFetch({ action: () => new Promise<Response>(resolve => { finish = resolve; }) });
 change('rolls-5', '8'); click('preview'); finish(new Response('{}')); await settle();
 live.topics['engineering.flow'] = { ...flowTopic(), request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,8] } }; flushSync(); await settle();
 expect(button('pin-goal').disabled).toBe(false);
 live.topics['engineering.flow'] = { ...flowTopic(), request: { ...fixture['engineering.flow'].request, rolls: [1,1,1,1,6] } }; flushSync(); await settle();
 expect((input('rolls-5') as HTMLInputElement).value).toBe('8');
 expect(button('preview').disabled).toBe(false);
 click('preview'); finish(new Response('{}')); await settle(); expect(actionCalls('preview')).toHaveLength(2);
});
test.each(['commander', 'ship'])('selection wait is invalidated after %s changes and ignores obsolete failure', async (changed) => {
 await showFlow(); const shipTarget = document.createElement('div'); document.body.append(shipTarget); const ship = mount(ShipPanel, { target: shipTarget }); flushSync();
 const contextChange = async () => {
  if (changed === 'commander') live.topics['core.cmdr'] = { name: 'Other' };
  else live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 9 };
  live.topics['engineering.flow'] = flowTopic(); flushSync(); await settle();
 };
 try {
  restFetch();
  const select = shipTarget.querySelector('[data-testid="module-FrameShiftDrive"]') as HTMLButtonElement;
  select.click(); await settle(); expect(button('preview').disabled).toBe(true);
  await contextChange(); expect(button('preview').disabled).toBe(false);
  // La primera selección no se confirma por WS; el segundo POST guarda esa espera anterior.
  select.click(); await settle();
  let reject!: (error: Error) => void; restFetch({ action: () => new Promise<Response>((_resolve, fail) => { reject = fail; }) });
  select.click(); flushSync();
  if (changed === 'commander') live.topics['core.cmdr'] = { name: 'Third' };
  else live.topics['engineering.ship'] = { ...fixture['engineering.ship'], ship_id: 10 };
  flushSync(); await settle(); expect(button('preview').disabled).toBe(true);
  reject(new Error('obsolete')); await settle();
  expect(button('preview').disabled).toBe(false);
  expect(button('close-flow').disabled).toBe(false);
  expect(button('pin-goal').disabled).toBe(false);
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
  if (url.startsWith('/api/engineering/engineers?')) return json({ ...restPages.engineers_engineers, revision: Number(new URL(url, 'http://localhost').searchParams.get('revision')), section: 'engineers' });
  const query = new URL(url, 'http://localhost').searchParams;
  const section = query.get('section')!;
  return new Response(JSON.stringify({ ...fixture['engineering.pages'].goals, revision: Number(query.get('revision')), section, items: section === 'goals' ? fixture['engineering.pages'].goals.items : [{ key: 'iron', count: 7 }] }));
 }));
}
function panel(component: typeof ShoppingPanel) { mounted = mount(component, { target }); flushSync(); }
test('goal shows percent blueprint grade missing and completable engineer', async () => {
 goalFetch(); panel(ShoppingPanel); await settle();
 expect(target.textContent).toContain('25 %'); expect(target.textContent).toContain('G5'); expect(target.textContent).toContain('Felicity Farseer'); expect(target.textContent).toContain('Sin verificar');
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
 goalFetch(); panel(EngineeringAlerts); await settle(); expect(target.textContent).toContain('Elvira Martuuk'); click('engineering-dismiss-0'); await settle();
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
test('all known engineering alert kinds resolve readable names and goals', async () => {
 goalFetch();
 live.topics['engineering.alerts']={alerts:[{id:'max',kind:'material_at_max',key:'iron',grade:1},{id:'invite',kind:'engineer_invited',engineer_id:300100},{id:'scarce',kind:'scarce_obtained',key:'iron'},{id:'goal',kind:'goal_completable',goal:fixture['engineering.pages'].goals.items[0].id,engineer:'Felicity Farseer'}]};
 panel(EngineeringAlerts); await settle(); expect(target.textContent).toContain('Hierro al máximo'); expect(target.textContent).toContain('Invitación nueva de Felicity Farseer'); expect(target.textContent).toContain('Material escaso conseguido'); expect(target.textContent).toContain('G5'); expect(target.textContent).not.toContain('fsd_longrange');
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
test('goal with materials ready but unknown Merc Coins balance is not completable and states why', async () => {
 const goal = fixture['engineering.pages'].goals.items[0];
 vi.stubGlobal('fetch', vi.fn(async (url: string) => {
  const query = new URL(url, 'http://localhost').searchParams; const section = query.get('section')!;
  const items = section === 'goals' ? [{ ...goal, completable: false, materials_ready: true, percent: 100, missing: {}, merc_coins: { need: 40, have: null, missing: null } }] : [];
  return json({ revision: Number(query.get('revision')), section, cursor: 0, next_cursor: null, total: items.length, items });
 }));
 panel(ShoppingPanel); await settle();
 expect(target.querySelector('[data-testid="merc-coins"]')?.textContent).toContain('Merc Coins necesarias: 40'); expect(target.querySelector('[data-testid="merc-coins"]')?.textContent).toContain('saldo sin datos');
 expect(target.querySelector('[data-testid="goal-merc-not-confirmed"]')).not.toBeNull(); expect(target.textContent).not.toContain('Ya puedes completarlo'); expect(target.textContent).toContain('Materiales: 100 % listo');
});
test('goal with calculation blockers lists the localized reasons', async () => {
 const goal = fixture['engineering.pages'].goals.items[0];
 vi.stubGlobal('fetch', vi.fn(async (url: string) => {
  const query = new URL(url, 'http://localhost').searchParams; const section = query.get('section')!;
  const items = section === 'goals' ? [{ ...goal, completable: false, blockers: [{ missing_recipe: { grade: 3 } }, 'base_required'] }] : [];
  return json({ revision: Number(query.get('revision')), section, cursor: 0, next_cursor: null, total: items.length, items });
 }));
 panel(ShoppingPanel); await settle();
 const reasons = target.querySelector('[data-testid="goal-blockers"]')!.textContent!; expect(reasons).toContain('Falta la receta de G3'); expect(reasons).toContain('base previa'); expect(reasons).not.toContain('base_required');
});

const capCalls = () => restCalls('engineers').filter(([url]) => String(url).includes('section=capabilities'));
test('capabilities are fetched only for the selected engineer by id', async () => {
 selectEngineer(); await showEngineers();
 expect(capCalls().map(([url]) => String(url))).toEqual(['/api/engineering/engineers?revision=21&cursor=0&section=capabilities&engineer_id=300100']);
});
test('a revision bump neither refetches nor blanks the capabilities', async () => {
 selectEngineer(); await showEngineers(); const before = capCalls().length;
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300100, next_step: 'Done', revision: 22 }; flushSync();
 expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('G1, G2, G3'); expect(target.querySelector('[data-testid="engineer-details"]')?.textContent).not.toContain('Cargando');
 await settle(); expect(capCalls()).toHaveLength(before);
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300100, next_step: { Requirement: { key: 'unlock-felicity-farseer' } }, revision: 23 }; flushSync(); await settle();
 expect(capCalls()).toHaveLength(before); expect(target.querySelector('[data-testid="engineer-next-step"]')?.textContent).toContain('Cumple el requisito');
});
test('changing the selected engineer fetches that engineer once and a changed catalogue invalidates the cache', async () => {
 selectEngineer(); await showEngineers();
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300160, next_step: 'Done' }; flushSync(); await settle();
 expect(capCalls().map(([url]) => new URL(String(url), 'http://x').searchParams.get('engineer_id'))).toEqual(['300100', '300160']);
 live.topics['engineering.catalogue'] = { ...fixture['engineering.catalogue'], counts: { ...fixture['engineering.catalogue'].counts, recipes: 99 } }; flushSync(); await settle();
 expect(capCalls()).toHaveLength(3);
});
test('a 409 while loading capabilities is retried with the new revision, not left stuck', async () => {
 selectEngineer(); let first = true;
 await showEngineers({ override: (resource, section) => resource === 'engineers' && section === 'capabilities' && first ? ((first = false), new Response('', { status: 409 })) : undefined });
 expect(target.textContent).toContain('Esperando los datos actualizados');
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], selected_id: 300100, next_step: 'Done', revision: 22 }; flushSync(); await settle();
 expect(capCalls()).toHaveLength(2); expect(target.querySelector('[data-testid="engineer-blueprints"]')?.textContent).toContain('G1, G2, G3');
});
test('alerts ask for each engineer name once by id and do not refetch on revision bumps', async () => {
 goalFetch(); panel(EngineeringAlerts); await settle();
 const named = () => vi.mocked(fetch).mock.calls.filter(([url]) => String(url).includes('engineer_id='));
 expect(named().map(([url]) => String(url))).toEqual(['/api/engineering/engineers?revision=21&cursor=0&section=engineers&engineer_id=300160']);
 expect(target.textContent).toContain('Elvira Martuuk');
 live.topics['engineering.engineers'] = { ...fixture['engineering.engineers'], revision: 22 }; flushSync(); await settle();
 expect(named()).toHaveLength(1); expect(target.textContent).toContain('Elvira Martuuk');
});
test('flow auto-correction of an inapplicable grade leaves the draft needing recalculation', async () => {
 await showFlow({ override: blueprintRows([{ key: 'engine_dirty', available: true, reason: null, grades: [2, 3, 4] }]) });
 expect((input('target-grade') as HTMLSelectElement).value).toBe('4');
 expect(target.textContent).toContain('Calcula los materiales para estos ajustes'); expect(target.querySelector('[data-testid="pin-goal"]')).toBeNull();
});
