import type { Component } from 'svelte';
import RouteStrip from './RouteStrip.svelte';
import JumpPanel from './JumpPanel.svelte';
import PlannerPanel from './PlannerPanel.svelte';

export const navigationPanels: Component[] = [RouteStrip, JumpPanel, PlannerPanel];

export { RouteStrip, JumpPanel, PlannerPanel };
