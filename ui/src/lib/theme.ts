export interface HudThemeInput {
  primary: string;
  secondary: string;
  background: string;
  danger: string;
}

function hexToRgba(hex: string, alpha: number): string {
  const clean = hex.trim().replace(/^#/, '');
  if (clean.length === 6) {
    const r = parseInt(clean.slice(0, 2), 16);
    const g = parseInt(clean.slice(2, 4), 16);
    const b = parseInt(clean.slice(4, 6), 16);
    if (!Number.isNaN(r) && !Number.isNaN(g) && !Number.isNaN(b)) {
      return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
  }
  return `color-mix(in srgb, ${hex} ${Math.round(alpha * 100)}%, transparent)`;
}

export function applyTheme(t: HudThemeInput): void {
  if (typeof document === 'undefined') return;
  const style = document.documentElement.style;
  style.setProperty('--hud-primary', t.primary);
  style.setProperty('--hud-secondary', t.secondary);
  style.setProperty('--hud-bg', t.background);
  style.setProperty('--hud-danger', t.danger);
  style.setProperty('--hud-primary-dim', hexToRgba(t.primary, 0.4));
  style.setProperty('--hud-border', hexToRgba(t.primary, 0.35));
}
