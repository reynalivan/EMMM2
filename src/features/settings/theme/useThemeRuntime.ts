import { useEffect } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import { normalizeThemeSetting, resolveTheme } from './themeOptions';

export function useThemeRuntime() {
  const { settings } = useSettings();
  const themeSetting = normalizeThemeSetting(settings?.theme);

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    const applyTheme = () => {
      const activeTheme = resolveTheme(themeSetting, media.matches);
      document.documentElement.setAttribute('data-theme', activeTheme);
    };

    applyTheme();

    if (themeSetting !== 'system') return;

    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
  }, [themeSetting]);
}
