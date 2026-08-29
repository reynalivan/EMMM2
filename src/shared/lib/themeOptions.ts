export const BUILTIN_THEMES = ['system', 'onyx', 'light'] as const;

export const THEME_OPTIONS = [
  { value: 'system', labelKey: 'general.appearance.themes.system' },
  { value: 'onyx', labelKey: 'general.appearance.themes.onyx' },
  { value: 'light', labelKey: 'general.appearance.themes.light' },
] as const;

export type ThemeSetting = string; // Allow custom strings

export type ResolvedTheme = string;

export function normalizeThemeSetting(value: string | null | undefined): ThemeSetting {
  if (!value) return 'onyx';
  return value;
}

export function resolveTheme(setting: ThemeSetting, prefersDark: boolean): ResolvedTheme {
  if (setting === 'system') return prefersDark ? 'onyx' : 'light';
  return setting;
}
