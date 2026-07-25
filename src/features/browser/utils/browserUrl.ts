import type { BrowserTab } from '../../../stores/useBrowserStore';

const LOADING_TITLE = 'Loading...';

/** Trim user input and prefix a scheme when the user typed a bare host. */
export function normalizeBrowserUrl(input: string): string {
  const trimmed = input.trim();
  if (trimmed.startsWith('http') || trimmed.startsWith('about:')) {
    return trimmed;
  }
  return `https://${trimmed}`;
}

/**
 * Label shown on a tab button: the page title once known, otherwise the host.
 * Returns null when neither is available so the caller can fall back to a
 * translated placeholder.
 */
export function tabDisplayLabel(tab: BrowserTab): string | null {
  if (tab.title && tab.title !== LOADING_TITLE) {
    return tab.title;
  }
  if (tab.url) {
    return new URL(tab.url).hostname;
  }
  return null;
}
