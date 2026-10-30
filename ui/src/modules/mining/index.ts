import type { Component } from 'svelte';
import ProspectorPanel from './ProspectorPanel.svelte';
import MiningSessionPanel from './MiningSessionPanel.svelte';
import SellPanel from './SellPanel.svelte';
import MiningTargetsPanel from './MiningTargetsPanel.svelte';

export const miningPanels: Component[] = [
  MiningTargetsPanel,
  ProspectorPanel,
  MiningSessionPanel,
  SellPanel,
];
export { ProspectorPanel, MiningSessionPanel, SellPanel, MiningTargetsPanel };
