import type { Component } from 'svelte';
import TargetPanel from './TargetPanel.svelte';
import ShipPanel from './ShipPanel.svelte';

export const combatPanels: Component[] = [TargetPanel, ShipPanel];
export { TargetPanel, ShipPanel };
