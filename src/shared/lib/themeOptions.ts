export const BUILTIN_THEMES = ['system', 'onyx', 'light'] as const;

export const THEME_OPTIONS = [
  { value: 'system', labelKey: 'general.appearance.themes.system' },
  { value: 'onyx', labelKey: 'general.appearance.themes.onyx' },
  { value: 'light', labelKey: 'general.appearance.themes.light' },
] as const;

export type ThemeSetting = string;

export type ResolvedTheme = string;

type CustomThemeListResult = {
  data?: readonly { id: string }[];
  isError: boolean;
  isFetching: boolean;
};

export function getConfirmedCustomThemeIds(result: CustomThemeListResult): string[] | undefined {
  if (result.isError || result.isFetching) return undefined;
  return result.data?.map((theme) => theme.id);
}

export function normalizeThemeSetting(
  value: string | null | undefined,
  customThemeIds?: readonly string[],
): ThemeSetting {
  if (!value) return 'onyx';
  // A persisted legacy selection is ambiguous until the custom list is available.
  if (value === 'dark' && customThemeIds && !customThemeIds.includes(value)) return 'onyx';
  return value;
}

export function getCustomThemeId(
  value: string | null | undefined,
  customThemeIds?: readonly string[],
): string | null {
  const theme = normalizeThemeSetting(value, customThemeIds);
  if (BUILTIN_THEMES.some((builtin) => builtin === theme)) return null;
  if (theme === 'dark' && !customThemeIds) return null;
  return theme;
}

export function resolveTheme(setting: ThemeSetting, prefersDark: boolean): ResolvedTheme {
  if (setting === 'system') return prefersDark ? 'onyx' : 'light';
  return setting;
}
