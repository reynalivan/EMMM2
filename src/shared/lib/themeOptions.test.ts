import { describe, expect, it } from 'vitest';
import { getCustomThemeId, normalizeThemeSetting, resolveTheme } from './themeOptions';

describe('theme settings', () => {
  it('uses onyx for a missing selection and resolves the system preference', () => {
    expect(normalizeThemeSetting(undefined)).toBe('onyx');
    expect(normalizeThemeSetting('')).toBe('onyx');
    expect(resolveTheme('system', true)).toBe('onyx');
    expect(resolveTheme('system', false)).toBe('light');
  });

  it('resolves legacy dark to onyx after custom themes have loaded', () => {
    expect(normalizeThemeSetting('dark', [])).toBe('onyx');
    expect(normalizeThemeSetting('dark', ['ocean-night'])).toBe('onyx');
  });

  it('preserves an existing custom dark theme and waits for unknown custom themes', () => {
    expect(normalizeThemeSetting('dark', ['dark'])).toBe('dark');
    expect(normalizeThemeSetting('dark')).toBe('dark');
    expect(normalizeThemeSetting('ocean-night', [])).toBe('ocean-night');
  });

  it('loads custom dark only after the authoritative list confirms it exists', () => {
    expect(getCustomThemeId('onyx')).toBeNull();
    expect(getCustomThemeId('light')).toBeNull();
    expect(getCustomThemeId('system')).toBeNull();
    expect(getCustomThemeId('dark')).toBeNull();
    expect(getCustomThemeId('dark', [])).toBeNull();
    expect(getCustomThemeId('dark', ['dark'])).toBe('dark');
    expect(getCustomThemeId('ocean-night')).toBe('ocean-night');
  });
});
