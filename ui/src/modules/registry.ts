import type { Component } from 'svelte';
import { dashboardPanels } from './dashboard';
import { navigationPanels } from './navigation';
import { explorationPanels } from './exploration';

export interface UiModuleDef {
  id: string;
  icon: string;
  titleKey: string;
  panels: Component[];
}

export const modules: UiModuleDef[] = [
  {
    id: 'dashboard',
    icon: 'gauge',
    titleKey: 'nav-dashboard',
    panels: dashboardPanels,
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
];
