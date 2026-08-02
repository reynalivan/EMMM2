import { useSyncExternalStore } from 'react';
import { useSettings } from './useSettings';
import { normalizeThemeSetting, resolveTheme } from '../lib/themeOptions';

const DARK_QUERY = '(prefers-color-scheme: dark)';

function subscribeToColorScheme(onChange: () => void) {
  const media = window.matchMedia(DARK_QUERY);
  media.addEventListener('change', onChange);
  return () => media.removeEventListener('change', onChange);
}

/**
 * The theme actually applied to the document, resolving `system` against the OS
 * preference and re-rendering when that preference flips. Anything that needs to
 * match the applied theme (chart palettes, canvas colours) must read it here
 * rather than re-deriving it — a one-shot `matchMedia(...).matches` at render
 * time goes stale as soon as the OS switches.
 */
export function useResolvedTheme(): string {
  const { settings } = useSettings();
  const prefersDark = useSyncExternalStore(
    subscribeToColorScheme,
    () => window.matchMedia(DARK_QUERY).matches,
  );
  return resolveTheme(normalizeThemeSetting(settings?.theme), prefersDark);
}
