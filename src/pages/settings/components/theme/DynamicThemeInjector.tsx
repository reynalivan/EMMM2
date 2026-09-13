import React, { useEffect } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useCustomTheme, useSettings } from '@/entities/settings';
import { type CustomTheme } from '../../../../shared/api/tauri/bindings';

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

  // QuickLiquid consumes a comma-separated RGB triplet; role-level tints are
  // resolved by the shared wrapper at render time.
  const baseTint = hexToRgb(config.colors?.['base-100']);
  if (baseTint) {
    css += `  --liquid-tint-rgb: ${baseTint};\n`;
  }

  const { background } = config;
  css += `  --theme-shell-dim-opacity: ${background.dim_opacity};\n`;
  if (background.kind === 'image') {
    css += `  --theme-shell-background-image: url("${convertFileSrc(background.value)}");\n`;
    css += `  --theme-shell-background-color: var(--color-base-100);\n`;
  } else if (background.kind === 'gradient') {
    css += `  --theme-shell-background-image: ${background.value};\n`;
    css += `  --theme-shell-background-color: var(--color-base-100);\n`;
  } else {
    css += `  --theme-shell-background-image: none;\n`;
    css += `  --theme-shell-background-color: ${background.value};\n`;
  }

  css += `}\n`;
  return css;
}

function hexToRgb(value: string | undefined): string | null {
  if (!value) return null;
  const hex = value.trim().replace(/^#/, '');
  if (!/^[\da-f]{3}([\da-f]{3})?$/i.test(hex)) return null;

  const normalized = hex.length === 3 ? [...hex].map((channel) => channel + channel).join('') : hex;
  return `${Number.parseInt(normalized.slice(0, 2), 16)}, ${Number.parseInt(normalized.slice(2, 4), 16)}, ${Number.parseInt(normalized.slice(4, 6), 16)}`;
}
