import type { Component } from 'svelte';
import AccountsPanel from './AccountsPanel.svelte';
import AccountsSection from './AccountsSection.svelte';

export const accountsPanels: Component[] = [AccountsPanel];
export { AccountsPanel, AccountsSection };
