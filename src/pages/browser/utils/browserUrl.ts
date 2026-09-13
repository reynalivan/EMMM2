import type { BrowserTab } from '@/entities/browser';

const LOADING_TITLE = 'Loading...';

export interface BrowserAddressParts {
  host: string;
  suffix: string;
}

/** Trim user input and prefix a scheme when the user typed a bare host. */
export function normalizeBrowserUrl(input: string): string {
  const trimmed = input.trim();
  if (trimmed.startsWith('http') || trimmed.startsWith('about:')) {
    return trimmed;
  }
  return `https://${trimmed}`;
}

/**
 * Split a navigable address into the identity users need to verify and the
 * lower-priority route detail. Keeping this as data (rather than styling an
 * input's value) lets the address field stay fully editable on focus.
 */
export function browserAddressParts(input: string): BrowserAddressParts | null {
  try {
    const parsed = new URL(input);
    if (!parsed.host) return null;

    return {
      host: parsed.host,
      suffix: `${parsed.pathname}${parsed.search}${parsed.hash}`,
    };
  } catch {
    return null;
  }
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
