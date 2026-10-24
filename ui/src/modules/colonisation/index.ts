import type { Component } from 'svelte';
import ColonisationProgressPanel from './ColonisationProgressPanel.svelte';
import ColonisationSupplyPanel from './ColonisationSupplyPanel.svelte';

export const colonisationPanels: Component[] = [ColonisationProgressPanel, ColonisationSupplyPanel];
export { ColonisationProgressPanel, ColonisationSupplyPanel };
