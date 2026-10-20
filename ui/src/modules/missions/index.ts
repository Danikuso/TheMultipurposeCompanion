import type { Component } from 'svelte';
import MissionsPanel from './MissionsPanel.svelte';
import MassacrePanel from './MassacrePanel.svelte';
import MissionAlerts from './MissionAlerts.svelte';

export const missionsPanels: Component[] = [MissionsPanel, MassacrePanel, MissionAlerts];
export { MissionsPanel, MassacrePanel, MissionAlerts };
