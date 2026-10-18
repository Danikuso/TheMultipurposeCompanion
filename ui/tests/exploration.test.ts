import { afterEach, beforeEach, expect, test } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale, t } from '../src/lib/i18n';
import { credits, num1 } from '../src/lib/format';
import SystemBodiesPanel from '../src/modules/exploration/SystemBodiesPanel.svelte';

let target:HTMLDivElement;
let mounted:ReturnType<typeof mount>|undefined;

function body(id:number,extra:Record<string,unknown>={}) {
  return {body_id:id,short_name:`${id} a`,kind_key:'rocky-ice',distance_ls:100.5,scanned_value:500,mapped_value:2708,...extra};
}
function system(rows:Record<string,unknown>[]) {
  return {system:'Synuefe Test',system_address:1,discovered:false,fss:{progress:0.5,body_count:600,scanned:rows.length,complete:false},value_now:1234567,value_if_mapped:2345678,unidentified:600-rows.length,rows};
}
function render(rows:Record<string,unknown>[]) {
  live.topics['exploration.system']=system(rows);
  mounted=mount(SystemBodiesPanel,{target});flushSync();
}
beforeEach(()=>{disconnect();setLocale('es');target=document.createElement('div');document.body.appendChild(target);});
afterEach(()=>{if(mounted)unmount(mounted);mounted=undefined;target.remove();});

test('body_table_renders_only_visible_rows',()=>{
  render(Array.from({length:300},(_,i)=>body(i+1)));
  const rows=target.querySelectorAll('[data-testid="body-row"]');
  expect(rows.length).toBeGreaterThan(0);expect(rows.length).toBeLessThanOrEqual(30);
  expect(target.querySelector('[data-testid="virtual-list"]')?.getAttribute('style')).toContain('600px');
});

test('map advice rows are highlighted and show probes',()=>{
  render([body(1,{advice:['map','land'],marks:3,bio:4,geo:3,probes:{n:6,estimated:true}})]);
  const row=target.querySelector('[data-testid="body-row"]')!;
  expect(row.classList.contains('map-advice')).toBe(true);
  expect(row.textContent).toContain(t('exploration-advice-map'));
  expect(row.textContent).toContain(t('exploration-advice-land'));
  expect(row.textContent).toContain(t('exploration-probes-estimated',{count:6}));
  expect(row.textContent).toContain(t('exploration-mark-terraformable'));
  expect(row.textContent).toContain(t('exploration-mark-landable'));
  expect(row.textContent).toContain(`${t('exploration-signals-bio')} 4`);
  expect(row.textContent).toContain(num1(100.5));
});

test('stars show dash in mapped column',()=>{
  render([body(1,{kind_key:'star-main',star_class:'K',mapped_value:undefined,marks:4})]);
  expect(target.querySelector('[data-testid="mapped-value"]')?.textContent?.trim()).toBe('—');
  expect(target.querySelector('[data-testid="body-kind"]')?.textContent).toContain('K');
  expect(target.textContent).toContain(t('exploration-mark-scoopable'));
});

test('footer shows unsold total and unidentified count',()=>{
  live.topics['exploration.unsold']={total:7654321,systems:3,at_risk:true};
  render([body(1)]);
  const footer=target.querySelector('[data-testid="exploration-footer"]')!;
  expect(footer.textContent).toContain(credits(7654321));
  expect(footer.textContent).toContain('599');
  expect(footer.textContent).toContain(t('exploration-at-risk'));
  expect(footer.textContent).toContain(t('exploration-unsold-systems',{count:3}));
  expect(target.querySelector('[data-testid="system-name"]')?.textContent).toBe('Synuefe Test');
  expect(target.querySelector('progress')?.value).toBe(0.5);
  expect(target.textContent).toContain(t('exploration-undiscovered'));
  expect(target.textContent).toContain(credits(1234567));
  expect(target.textContent).toContain(credits(2345678));
});

test.each(['es','en','fr','de','pt'])('truncated bodies show a localized count separately from FSS in %s',locale=>{
  setLocale(locale);
  live.topics['exploration.system']={...system([body(1)]),truncated:1234,unidentified:7};
  mounted=mount(SystemBodiesPanel,{target});flushSync();
  const notice=target.querySelector('[data-testid="exploration-truncated"]');
  expect(notice).not.toBeNull();
  const message=t('exploration-truncated',{count:1234});
  expect(message).not.toBe('exploration-truncated');
  expect(notice?.textContent).toBe(message);
  expect(notice?.textContent).toContain(new Intl.NumberFormat(locale).format(1234));
  expect(target.querySelector('[data-testid="exploration-footer"]')?.textContent).toContain(`${t('exploration-unidentified')}: 7`);
  setLocale(locale==='es'?'de':'es');flushSync();
  expect(notice?.textContent).toBe(t('exploration-truncated',{count:1234}));
});

test.each([undefined,0])('no truncated notice for count %s',truncated=>{
  live.topics['exploration.system']={...system([body(1)]),truncated};
  mounted=mount(SystemBodiesPanel,{target});flushSync();
  expect(target.querySelector('[data-testid="exploration-truncated"]')).toBeNull();
  expect(target.querySelector('[data-testid="exploration-footer"]')?.textContent).toContain(`${t('exploration-unidentified')}: 599`);
});

test('scroll updates visible rows and shrinking items clamps to real rows',()=>{
  render(Array.from({length:300},(_,i)=>body(i+1)));
  const viewport=target.querySelector('[data-testid="virtual-list"]') as HTMLDivElement;
  viewport.scrollTop=6000;viewport.dispatchEvent(new Event('scroll'));flushSync();
  expect(target.querySelector('[data-testid="body-row"]')?.getAttribute('data-body-id')).toBe('196');
  expect(target.querySelectorAll('[data-testid="body-row"]').length).toBeLessThanOrEqual(30);
  live.topics['exploration.system']=system([body(999)]);flushSync();
  expect(target.querySelectorAll('[data-testid="body-row"]').length).toBe(1);
  expect(target.querySelector('[data-testid="body-row"]')?.getAttribute('data-body-id')).toBe('999');
  expect(viewport.scrollTop).toBe(0);
});

test('compact defaults and exact probes preserve all marks and advice',()=>{
  render([body(1,{marks:63,probes:{n:6}}),body(2)]);
  const rows=target.querySelectorAll('[data-testid="body-row"]');
  expect(rows[0].textContent).toContain(t('exploration-probes-exact',{count:6}));
  expect(rows[0].textContent).not.toContain('≈');
  expect(rows[0].textContent).toContain(t('exploration-advice-not-worth'));
  for(const key of ['terraformable','landable','scoopable','first-discovery','already-mapped','mapped-by-you']) expect(rows[0].textContent).toContain(t(`exploration-mark-${key}`));
  expect(rows[1].querySelector('[data-testid="body-marks"]')?.textContent?.trim()).toBe('—');
});

test('missing system topic has a localized empty state',()=>{
  mounted=mount(SystemBodiesPanel,{target});flushSync();
  expect(target.textContent).toContain(t('empty-no-data'));
  expect(target.querySelectorAll('[data-testid="body-row"]').length).toBe(0);
});

test('each compact mark bit maps to its own label',()=>{
  const keys=['terraformable','landable','scoopable','first-discovery','already-mapped','mapped-by-you'];
  render(keys.map((_,index)=>body(index+1,{marks:1<<index})));
  const rows=target.querySelectorAll('[data-testid="body-marks"]');
  for(let index=0;index<keys.length;index++) {
    expect(rows[index].textContent?.trim()).toBe(t(`exploration-mark-${keys[index]}`));
  }
});

test('all bodies found completes the FSS bar and unknown values stay unknown',()=>{
  live.topics['exploration.system']={...system([body(1,{distance_ls:undefined,mapped_value:undefined})]),fss:{scanned:1,complete:true}};
  mounted=mount(SystemBodiesPanel,{target});flushSync();
  expect(target.querySelector('progress')?.value).toBe(1);
  expect(target.querySelector('[data-testid="mapped-value"]')?.textContent?.trim()).toBe('—');
  expect(target.textContent).toContain(t('exploration-scanned-count',{scanned:1,total:'—'}));
});

test('locale changes update existing rows and number formatting',()=>{
  render([body(1,{mapped_value:1500000,probes:{n:5,estimated:true}})]);
  setLocale('de');flushSync();
  expect(target.textContent).toContain(t('panel-system-bodies'));
  expect(target.textContent).toContain(t('exploration-kind-rocky-ice'));
  expect(target.textContent).toContain(credits(1500000));
});
