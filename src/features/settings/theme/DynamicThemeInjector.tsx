import React, { useEffect } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import { type CustomTheme } from '../../../lib/bindings';
import { useCustomTheme } from './useCustomThemes';

const BUILTIN_THEME_IDS = new Set(['onyx', 'light', 'system']);

/**
 * DynamicThemeInjector
 *
 * Injects custom CSS variables into the document head when a non-builtin theme is selected.
 */
export const DynamicThemeInjector: React.FC = () => {
  const { settings } = useSettings();
  const theme = settings?.theme;
  const isBuiltin = !theme || BUILTIN_THEME_IDS.has(theme);
  const { data: customTheme, isError } = useCustomTheme(isBuiltin ? null : theme);

  useEffect(() => {
    const existingStyle = document.getElementById('dynamic-theme-style');

    if (isBuiltin || isError) {
      if (isError) {
        console.error(`[DynamicTheme] Failed to load custom theme "${theme}"`);
      }
      if (existingStyle) {
        existingStyle.innerHTML = '';
      }
      return;
    }

    if (!customTheme) {
      return;
    }

    let styleTag = existingStyle as HTMLStyleElement | null;
    if (!styleTag) {
      styleTag = document.createElement('style');
      styleTag.id = 'dynamic-theme-style';
      document.head.appendChild(styleTag);
    }
    styleTag.innerHTML = generateThemeCss(customTheme);
  }, [customTheme, isBuiltin, isError, theme]);

  return null;
};

/**
 * Generates CSS variable overrides for a custom theme.
 * Reuses the same semantic variable names defined in App.css.
 */
function generateThemeCss(theme: CustomTheme): string {
  const { id, config } = theme;
  let css = `[data-theme="${id}"] {\n`;

  // Custom Color Overrides
  if (config.colors) {
    for (const [key, value] of Object.entries(config.colors)) {
      css += `  --color-${key}: ${value};\n`;
      // Handle DaisyUI specifics if needed (though we use Tailwind 4 variables mostly)
      if (key === 'primary') css += `  --p: ${value};\n`;
      if (key === 'secondary') css += `  --s: ${value};\n`;
      if (key === 'accent') css += `  --a: ${value};\n`;
      if (key === 'neutral') css += `  --n: ${value};\n`;
      if (key === 'base-100') css += `  --b1: ${value};\n`;
    }
  }

  // Glass Overrides
  if (config.glass) {
    if (config.glass.bg) css += `  --glass-bg: ${config.glass.bg};\n`;
    if (config.glass.border) css += `  --glass-border: ${config.glass.border};\n`;
  }

  // Default Glass behavior if missing
  if (!config.glass?.bg) {
    // Fallback: semi-transparent base-100
    css += `  --glass-bg: color-mix(in srgb, var(--color-base-100) 40%, transparent);\n`;
  }

  css += `}\n`;
  return css;
}
