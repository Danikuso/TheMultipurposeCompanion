import type { Component } from 'svelte';
import RouteStrip from './RouteStrip.svelte';
import JumpPanel from './JumpPanel.svelte';

export const navigationPanels: Component[] = [RouteStrip, JumpPanel];

export { RouteStrip, JumpPanel };
