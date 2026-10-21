import type { Component } from 'svelte';
import MaterialsPanel from './MaterialsPanel.svelte';
import EngineersPanel from './EngineersPanel.svelte';
import ShipPanel from './ShipPanel.svelte';
import EngineerFlow from './EngineerFlow.svelte';
export const engineeringPanels: Component[] = [MaterialsPanel, EngineersPanel, ShipPanel, EngineerFlow];
export { MaterialsPanel, EngineersPanel, ShipPanel, EngineerFlow };
