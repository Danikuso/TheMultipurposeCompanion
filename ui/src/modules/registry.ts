import type { Component } from 'svelte';
import { dashboardPanels } from './dashboard';

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
];
