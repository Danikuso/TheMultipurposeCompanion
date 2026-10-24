import type { Component } from 'svelte';
import { dashboardPanels, ShipPanel } from './dashboard';
import { navigationPanels } from './navigation';
import { explorationPanels } from './exploration';
import { exobiologyPanels } from './exobiology';
import { combatPanels } from './combat';
import { miningPanels } from './mining';
import { tradePanels } from './trade';
import { missionsPanels } from './missions';
import { engineeringPanels } from './engineering';
import { carrierPanels } from './carrier';
import { colonisationPanels } from './colonisation';
import { powerplayPanels } from './powerplay';
import { bgsPanels } from './bgs';
import { accountsPanels } from './accounts';

export interface UiModuleDef {
  id: string;
  icon: string;
  titleKey: string;
  panels: Component[];
}

export const modules: UiModuleDef[] = [
  { id: 'engineering', icon: 'wrench', titleKey: 'nav-engineering', panels: engineeringPanels },
  {
    id: 'dashboard',
    icon: 'gauge',
    titleKey: 'nav-dashboard',
    panels: dashboardPanels,
  },
  {
    id: 'ship',
    icon: 'rocket',
    titleKey: 'nav-ship',
    panels: [ShipPanel],
  },
  {
    id: 'navigation',
    icon: 'route',
    titleKey: 'nav-navigation',
    panels: navigationPanels,
  },
  {
    id: 'exploration',
    icon: 'orbit',
    titleKey: 'nav-exploration',
    panels: explorationPanels,
  },
  {
    id: 'exobiology',
    icon: 'leaf',
    titleKey: 'nav-exobiology',
    panels: exobiologyPanels,
  },
  {
    id: 'combat',
    icon: 'crosshair',
    titleKey: 'nav-combat',
    panels: combatPanels,
  },
  {
    id: 'mining',
    icon: 'pickaxe',
    titleKey: 'nav-mining',
    panels: miningPanels,
  },
  {
    id: 'trade',
    icon: 'coins',
    titleKey: 'nav-trade',
    panels: tradePanels,
  },
  {
    id: 'missions',
    icon: 'clipboard-list',
    titleKey: 'nav-missions',
    panels: missionsPanels,
  },
  {
    id: 'carrier',
    icon: 'anchor',
    titleKey: 'nav-carrier',
    panels: carrierPanels,
  },
  {
    id: 'colonisation',
    icon: 'hammer',
    titleKey: 'nav-colonisation',
    panels: colonisationPanels,
  },
  {
    id: 'powerplay',
    icon: 'crown',
    titleKey: 'nav-powerplay',
    panels: powerplayPanels,
  },
  {
    id: 'bgs',
    icon: 'network',
    titleKey: 'nav-bgs',
    panels: bgsPanels,
  },
  {
    id: 'accounts',
    icon: 'globe',
    titleKey: 'nav-accounts',
    panels: accountsPanels,
  },
];
