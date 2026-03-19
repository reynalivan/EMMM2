export const THEME_OPTIONS = [
  { value: 'system', label: 'System Default' },
  { value: 'dark', label: 'Dark (Dracula)' },
  { value: 'light', label: 'Light' },
  { value: 'cyberpunk', label: 'Cyberpunk' },
  { value: 'onyx', label: 'Onyx (EMMM2)' },
] as const;

export type ThemeSetting = (typeof THEME_OPTIONS)[number]['value'];

export type ResolvedTheme = Exclude<ThemeSetting, 'system'>;

export function isThemeSetting(value: string): value is ThemeSetting {
  return THEME_OPTIONS.some((option) => option.value === value);
}

export function normalizeThemeSetting(value: string | null | undefined): ThemeSetting {
  if (!value) return 'dark';
  return isThemeSetting(value) ? value : 'dark';
}

export function resolveTheme(setting: ThemeSetting, prefersDark: boolean): ResolvedTheme {
  if (setting === 'system') return prefersDark ? 'dark' : 'light';
  return setting;
}
