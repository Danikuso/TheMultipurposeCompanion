import { afterEach, beforeEach, expect, test } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { disconnect, live } from '../src/lib/ws.svelte';
import { setLocale } from '../src/lib/i18n';
import ExobiologyPanel from '../src/modules/exobiology/ExobiologyPanel.svelte';

let target:HTMLDivElement;
let mounted:ReturnType<typeof mount>|undefined;

function candidate(extra:Record<string,unknown>={}) {
  return {species:'$Codex_Ent_Bacterial_01_Name;',species_name:'Bacterium Aurasus',genus:'$Codex_Ent_Bacterial_Genus_Name;',genus_name:'Bacterium',
    value:1000000,value_x5:5000000,min_m:500,likelihood:'high',sources:['rules'],status:'none',progress:null,...extra};
}
function bodyTopic(extra:Record<string,unknown>={}) {
  return {body:'Kappa Testis A 5 a',body_id:5,system_address:1,kind:'Rocky body',gravity_g:0.12,temperature_k:170,atmosphere:'CarbonDioxide',
    bio_signals:3,first_logged:true,dss:true,candidates:[candidate()],confirmed_genera:[{genus:'g1',name:'Bacterium'},{genus:'g2',name:'Stratum'}],completed:1,...extra};
}
function samplingTopic(extra:Record<string,unknown>={}) {
  return {species:'$Codex_Ent_Bacterial_01_Name;',species_name:'Bacterium Aurasus',genus:'g',genus_name:'Bacterium',body_id:5,points:2,complete:false,
    distance_m:262,min_m:500,remaining_m:238,direction:{rel:'behind',compass:'SW'},unsold_after:5000000,discarded_species:null,discarded_name:null,...extra};
}
function render(body:unknown,sampling?:unknown,unsold:unknown={total:1000000,species:1}) {
  if(body) live.topics['exobiology.body']=body;
  if(sampling) live.topics['exobiology.sampling']=sampling;
  if(unsold) live.topics['exobiology.unsold']=unsold;
  mounted=mount(ExobiologyPanel,{target});flushSync();
}
beforeEach(()=>{disconnect();setLocale('es');target=document.createElement('div');document.body.appendChild(target);});
afterEach(()=>{if(mounted)unmount(mounted);mounted=undefined;target.remove();});

test('candidates show likelihood and its origin',()=>{
  render(bodyTopic({candidates:[
    candidate({status:'in-progress',progress:2,sources:['rules','history','canonn']}),
    candidate({species_name:'Osseus Discus',likelihood:'unknown',sources:[],status:'done',progress:3}),
    candidate({species_name:'Tussock Ventusa',likelihood:'low',sources:['rules'],value:3300000}),
  ]}));
  const rows=target.querySelectorAll('[data-testid="candidate-row"]');
  expect(rows.length).toBe(3);
  expect(rows[0].textContent).toContain('Bacterium Aurasus');
  expect(rows[0].textContent).toContain('1.000.000 CR');
  expect(rows[0].textContent).toContain('5.000.000 CR');
  expect(rows[0].textContent).toContain('500 m');
  expect(rows[0].textContent).toContain('Alta');
  expect(rows[0].textContent).toContain('reglas · tu historial · Canonn');
  expect(rows[0].textContent).toContain('▶ 2/3');
  expect(rows[1].textContent).toContain('Desconocida');
  expect(rows[1].textContent).toContain('✓ 3/3');
  expect(rows[2].textContent).toContain('Baja');
  expect(rows[2].textContent).toContain('—');
  const genera=target.querySelector('[data-testid="genera"]')!;
  expect(genera.textContent).toContain('Bacterium, Stratum');
  expect(genera.textContent).toContain('1 de 3');
  const header=target.querySelector('[data-testid="body-header"]')!;
  expect(header.textContent).toContain('Kappa Testis A 5 a');
  expect(header.textContent).toContain('0,12 g');
  expect(header.textContent).toContain('170 K');
  expect(header.textContent).toContain('3 señales biológicas');
});

test('sampling bar shows distance versus minimum and remaining',()=>{
  render(bodyTopic(),samplingTopic());
  const s=target.querySelector('[data-testid="sampling"]')!;
  expect(s.textContent).toContain('Bacterium Aurasus');
  expect(s.textContent).toContain('2 de 3 muestras');
  expect(s.textContent).toContain('262 m / 500 m');
  expect(s.textContent).toContain('Te faltan 238 m');
  expect((target.querySelector('[data-testid="distance-bar"]') as HTMLElement).style.width).toBe('52.4%');
  expect(s.textContent).toContain('5.000.000 CR');
  expect(target.querySelector('[data-testid="unsold"]')?.textContent).toContain('1.000.000 CR');
});

test('sampling distance bar is capped and handles missing distance',()=>{
  render(bodyTopic(),samplingTopic({distance_m:900,remaining_m:0}));
  expect((target.querySelector('[data-testid="distance-bar"]') as HTMLElement).style.width).toBe('100%');
  unmount(mounted!);target.innerHTML='';
  live.topics['exobiology.sampling']=samplingTopic({distance_m:null,remaining_m:null,direction:null});
  mounted=mount(ExobiologyPanel,{target});flushSync();
  expect(target.querySelector('[data-testid="sampling"]')?.textContent).toContain('sin datos');
  expect((target.querySelector('[data-testid="distance-bar"]') as HTMLElement).style.width).toBe('0%');
});

test('direction text uses relative and compass keys',()=>{
  render(bodyTopic(),samplingTopic());
  expect(target.querySelector('[data-testid="direction"]')?.textContent).toBe('la anterior está a tu espalda (SO)');
});

test('first logged badge is marked as estimated',()=>{
  render(bodyTopic({first_logged:true}));
  expect(target.querySelector('[data-testid="first-logged"]')?.textContent).toContain('SIN REGISTRAR · ×5 (estimado)');
  unmount(mounted!);target.innerHTML='';
  live.topics['exobiology.body']=bodyTopic({first_logged:false});
  mounted=mount(ExobiologyPanel,{target});flushSync();
  expect(target.querySelector('[data-testid="first-logged"]')).toBeNull();
});

test('discarded species is shown in the sampling block',()=>{
  render(bodyTopic(),samplingTopic({points:1,discarded_species:'$x;',discarded_name:'Bacterium Aurasus'}));
  expect(target.querySelector('[data-testid="discarded"]')?.textContent).toBe('Muestreo descartado: Bacterium Aurasus');
});

test('without sampling the block waits for ScanOrganic',()=>{
  render(bodyTopic());
  expect(target.querySelector('[data-testid="sampling"]')?.textContent).toContain('Esperando ScanOrganic…');
});

test('missing body topic shows the empty state',()=>{
  mounted=mount(ExobiologyPanel,{target});flushSync();
  expect(target.querySelector('[data-testid="empty"]')).not.toBeNull();
  expect(target.querySelector('[data-testid="candidate-row"]')).toBeNull();
});

test('numbers and labels follow the locale',()=>{
  setLocale('en');
  render(bodyTopic(),samplingTopic());
  expect(target.querySelector('[data-testid="body-header"]')?.textContent).toContain('0.12 g');
  expect(target.querySelector('[data-testid="candidate-row"]')?.textContent).toContain('5,000,000 CR');
  expect(target.querySelector('[data-testid="direction"]')?.textContent).toBe('the previous one is behind you (SW)');
  expect(target.querySelector('[data-testid="candidate-row"]')?.textContent).toContain('High');
});
