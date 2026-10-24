import type { Component } from 'svelte';
import CarrierOverviewPanel from './CarrierOverviewPanel.svelte';
import CarrierJumpPanel from './CarrierJumpPanel.svelte';
import CarrierFinancesPanel from './CarrierFinancesPanel.svelte';

export const carrierPanels: Component[] = [CarrierOverviewPanel, CarrierJumpPanel, CarrierFinancesPanel];
export { CarrierOverviewPanel, CarrierJumpPanel, CarrierFinancesPanel };
