import { useEffect } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import { normalizeThemeSetting, resolveTheme, BUILTIN_THEMES } from './themeOptions';

export function useThemeRuntime() {
  const { settings } = useSettings();
  const themeSetting = normalizeThemeSetting(settings?.theme);

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    const applyTheme = () => {
      const activeTheme = resolveTheme(themeSetting, media.matches);

      // DaisyUI 5 Mechanism: Set data-theme attribute
      document.documentElement.setAttribute('data-theme', activeTheme);

      // Sync CSS class for Tailwind 4 @theme activation
      // Remove all previous builtin theme classes to avoid variable pollution
      BUILTIN_THEMES.forEach((t) => {
        document.documentElement.classList.remove(t);
      });

      // Add current theme class (Tailwind 4 uses this to activate @theme blocks)
      document.documentElement.classList.add(activeTheme);
    };

    applyTheme();

    if (themeSetting !== 'system') return;

    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
  }, [themeSetting]);
}
