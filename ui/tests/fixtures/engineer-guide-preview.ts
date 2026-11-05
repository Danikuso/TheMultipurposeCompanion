// Isolated visual QA fixture: synthetic commander, generated Rust contract, no backend or network.
import '../../src/styles/base.css';
import { mount } from 'svelte';
import EngineerGuidePanel from '../../src/modules/engineering/EngineerGuidePanel.svelte';
import { live } from '../../src/lib/ws.svelte';
import { setLocale } from '../../src/lib/i18n';
import fixture from './engineering-topics.json';
import type { EngineerRow, EngineersTopic, EngineerUnlockRow } from '../../src/modules/engineering/types';

const query = new URLSearchParams(location.search);
setLocale(query.get('lang') ?? 'es');
const id = Number(query.get('id') ?? 300100);
const rows = structuredClone(fixture['engineering.pages'].engineers_engineers.items) as EngineerRow[];
const unlock = fixture['engineering.pages'].engineers_unlock.items as EngineerUnlockRow[];
const selected = rows.find(row => row.id === id)!;
selected.status = query.get('status') ?? 'Known';
selected.rank = selected.status === 'Unlocked' ? Number(query.get('rank') ?? 3) : null;
selected.rank_progress = selected.status === 'Unlocked' ? Number(query.get('progress') ?? 42) : null;
live.connected = true;
live.topics['core.cmdr'] = { name: 'Synthetic', credits: 500000 };
live.topics['core.ship'] = { cargo: [{ Name: 'metaalloys', Count: Number(query.get('stock') ?? 3) }] };
live.topics['engineering.inventory'] = fixture['engineering.inventory'];
live.topics['engineering.engineers'] = { revision: 21, total: 38, selected_id: id, next_step: null };
window.fetch = async (input, init) => {
  const current = live.topics['engineering.engineers'] as EngineersTopic;
  if (init?.method === 'POST') {
    const payload = JSON.parse(String(init.body));
    live.topics['engineering.engineers'] = { ...current, selected_id: payload.id, revision: current.revision + 1 };
    return new Response('{}');
  }
  const params = new URL(String(input), location.origin).searchParams;
  const section = params.get('section');
  const items = section === 'engineers' ? rows : unlock.filter(row => row.engineer_id === Number(params.get('engineer_id')));
  return new Response(JSON.stringify({ revision: Number(params.get('revision')), section, cursor: 0, total: items.length, next_cursor: null, items }));
};
mount(EngineerGuidePanel, { target: document.getElementById('preview')! });
