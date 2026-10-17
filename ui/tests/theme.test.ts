import { describe, expect, test } from 'vitest';
import { applyTheme } from '../src/lib/theme';

describe('theme', () => {
  test('applyTheme sets css variables', () => {
    applyTheme({
      primary: '#ff7100',
      secondary: '#00a2ff',
      background: '#0b0c10',
      danger: '#ff2b2b',
    });
    const style = document.documentElement.style;
    expect(style.getPropertyValue('--hud-primary')).toBe('#ff7100');
    expect(style.getPropertyValue('--hud-secondary')).toBe('#00a2ff');
    expect(style.getPropertyValue('--hud-bg')).toBe('#0b0c10');
    expect(style.getPropertyValue('--hud-danger')).toBe('#ff2b2b');
    expect(style.getPropertyValue('--hud-primary-dim')).not.toBe('');
    expect(style.getPropertyValue('--hud-border')).not.toBe('');
  });
});
