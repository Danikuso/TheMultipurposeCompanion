import type { Component } from 'svelte';
import MarketPanel from './MarketPanel.svelte';
import CargoPanel from './CargoPanel.svelte';
import RoutePanel from './RoutePanel.svelte';
import TradeSessionPanel from './TradeSessionPanel.svelte';
import SearchPanel from './SearchPanel.svelte';

export const tradePanels: Component[] = [MarketPanel, CargoPanel, RoutePanel, TradeSessionPanel, SearchPanel];
export { MarketPanel, CargoPanel, RoutePanel, TradeSessionPanel, SearchPanel };
