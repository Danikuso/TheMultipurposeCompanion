import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setLocale } from '../src/lib/i18n';
import TopBar from '../src/shell/TopBar.svelte';
import Sidebar from '../src/shell/Sidebar.svelte';

let target: HTMLDivElement;
const mounted: Array<ReturnType<typeof mount>> = [];

beforeEach(() => {
  setLocale('es');
  target = document.createElement('div');
  document.body.appendChild(target);
});

afterEach(() => {
  while (mounted.length > 0) {
    const inst = mounted.pop();
    if (inst) unmount(inst);
  }
  target.remove();
});

describe('Responsive mobile navigation (< 768 px)', () => {
  test('TopBar renders mobile-nav-toggle button and calls onToggleMobileNav', () => {
    const onToggleMobileNav = vi.fn();
    mounted.push(
      mount(TopBar, {
        target,
        props: { mobileNavOpen: false, onToggleMobileNav },
      }),
    );
    flushSync();

    const btn = target.querySelector('[data-testid="mobile-nav-toggle"]') as HTMLButtonElement;
    expect(btn).toBeTruthy();
    expect(btn.getAttribute('aria-expanded')).toBe('false');

    btn.click();
    flushSync();
    expect(onToggleMobileNav).toHaveBeenCalledTimes(1);
  });

  test('Sidebar toggles mobile-open class and backdrop when mobileOpen is true', () => {
    const onCloseMobile = vi.fn();
    const closedHost = document.createElement('div');
    target.appendChild(closedHost);
    mounted.push(
      mount(Sidebar, {
        target: closedHost,
        props: { mobileOpen: false, onCloseMobile },
      }),
    );
    flushSync();

    const closedAside = closedHost.querySelector('aside.hud-sidebar');
    expect(closedAside?.classList.contains('mobile-open')).toBe(false);
    expect(closedHost.querySelector('[data-testid="sidebar-mobile-backdrop"]')).toBeNull();

    const openHost = document.createElement('div');
    target.appendChild(openHost);
    mounted.push(
      mount(Sidebar, {
        target: openHost,
        props: { mobileOpen: true, onCloseMobile },
      }),
    );
    flushSync();

    const openAside = openHost.querySelector('aside.hud-sidebar');
    expect(openAside?.classList.contains('mobile-open')).toBe(true);
    const backdrop = openHost.querySelector('[data-testid="sidebar-mobile-backdrop"]') as HTMLElement;
    expect(backdrop).toBeTruthy();

    backdrop.click();
    flushSync();
    expect(onCloseMobile).toHaveBeenCalledTimes(1);
  });
});
