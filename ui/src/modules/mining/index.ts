import type { Component } from 'svelte';
import ProspectorPanel from './ProspectorPanel.svelte';
import MiningSessionPanel from './MiningSessionPanel.svelte';
import SellPanel from './SellPanel.svelte';

export const miningPanels: Component[] = [ProspectorPanel, MiningSessionPanel, SellPanel];
export { ProspectorPanel, MiningSessionPanel, SellPanel };
