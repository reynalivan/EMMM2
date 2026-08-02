import { useEffect } from 'react';
import { useResolvedTheme } from '../../../hooks/useResolvedTheme';
import { BUILTIN_THEMES } from '../../../lib/themeOptions';

/** Applies the resolved theme to the document. Mounted once, at the app shell. */
export function useThemeRuntime() {
  const activeTheme = useResolvedTheme();

  useEffect(() => {
    // DaisyUI 5 Mechanism: Set data-theme attribute
    document.documentElement.setAttribute('data-theme', activeTheme);

    // Sync CSS class for Tailwind 4 @theme activation
    // Remove all previous builtin theme classes to avoid variable pollution
    BUILTIN_THEMES.forEach((t) => {
      document.documentElement.classList.remove(t);
    });

    // Add current theme class (Tailwind 4 uses this to activate @theme blocks)
    document.documentElement.classList.add(activeTheme);
  }, [activeTheme]);
}
