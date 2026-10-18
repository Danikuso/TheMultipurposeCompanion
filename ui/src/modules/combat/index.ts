import type { Component } from 'svelte';
import TargetPanel from './TargetPanel.svelte';
import ShipPanel from './ShipPanel.svelte';
import HuntingPanel from './HuntingPanel.svelte';
import ConflictZonePanel from './ConflictZonePanel.svelte';
import AxPanel from './AxPanel.svelte';

export const combatPanels: Component[] = [TargetPanel, ShipPanel, HuntingPanel, ConflictZonePanel, AxPanel];
export { TargetPanel, ShipPanel, HuntingPanel, ConflictZonePanel, AxPanel };
