import type { Component } from 'svelte';
import CmdrPanel from './CmdrPanel.svelte';
import ShipPanel from './ShipPanel.svelte';
import LocationPanel from './LocationPanel.svelte';
import StatusPanel from './StatusPanel.svelte';
import RoutePanel from './RoutePanel.svelte';
import SessionPanel from './SessionPanel.svelte';
import AlertsPanel from './AlertsPanel.svelte';
import OutfittingSearchPanel from './OutfittingSearchPanel.svelte';

export const dashboardPanels: Component[] = [
  CmdrPanel,
  ShipPanel,
  LocationPanel,
  StatusPanel,
  RoutePanel,
  SessionPanel,
  AlertsPanel,
];

export {
  CmdrPanel,
  ShipPanel,
  OutfittingSearchPanel,
  LocationPanel,
  StatusPanel,
  RoutePanel,
  SessionPanel,
  AlertsPanel,
};
