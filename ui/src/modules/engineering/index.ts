import type { Component } from 'svelte';
import MaterialsPanel from './MaterialsPanel.svelte';
import EngineersPanel from './EngineersPanel.svelte';
export const engineeringPanels: Component[] = [MaterialsPanel, EngineersPanel];
export { MaterialsPanel, EngineersPanel };
