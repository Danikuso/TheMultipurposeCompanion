import type { Component } from 'svelte';
import MissionsPanel from './MissionsPanel.svelte';
import MassacrePanel from './MassacrePanel.svelte';
import MissionAlerts from './MissionAlerts.svelte';
import PassengersPanel from './PassengersPanel.svelte';
import TourismPanel from './TourismPanel.svelte';
import RewardsPanel from './RewardsPanel.svelte';
import CommunityGoalsPanel from './CommunityGoalsPanel.svelte';

export const missionsPanels: Component[] = [
  MissionsPanel,
  MassacrePanel,
  MissionAlerts,
  PassengersPanel,
  TourismPanel,
  RewardsPanel,
  CommunityGoalsPanel,
];
export { MissionsPanel, MassacrePanel, MissionAlerts, PassengersPanel, TourismPanel, RewardsPanel, CommunityGoalsPanel };
