import type { Component } from 'svelte';
import MarketPanel from './MarketPanel.svelte';
import CargoPanel from './CargoPanel.svelte';

export const tradePanels: Component[] = [MarketPanel, CargoPanel];
export { MarketPanel, CargoPanel };
