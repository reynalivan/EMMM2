import React, { useEffect } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import { commands, type CustomTheme } from '../../../lib/bindings';

/**
 * DynamicThemeInjector
 *
 * Injects custom CSS variables into the document head when a non-builtin theme is selected.
 */
export const DynamicThemeInjector: React.FC = () => {
  const { settings } = useSettings();
  const theme = settings?.theme;

  useEffect(() => {
    const existingStyle = document.getElementById('dynamic-theme-style');

    // Built-in themes don't need injection
    if (theme === 'onyx' || theme === 'light' || theme === 'system') {
      if (existingStyle) {
        existingStyle.innerHTML = '';
      }
      return;
    }

    const loadAndInject = async () => {
      try {
        if (!theme) return;
        const customTheme = await commands.loadCustomTheme({ id: theme });
        const css = generateThemeCss(customTheme);

        let styleTag = existingStyle as HTMLStyleElement;
        if (!styleTag) {
          styleTag = document.createElement('style');
          styleTag.id = 'dynamic-theme-style';
          document.head.appendChild(styleTag);
        }
        styleTag.innerHTML = css;
      } catch (err) {
        console.error(`[DynamicTheme] Failed to load custom theme "${theme}":`, err);
        // Fallback or cleanup
        if (existingStyle) existingStyle.innerHTML = '';
      }
    };

    loadAndInject();
  }, [theme]);

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
