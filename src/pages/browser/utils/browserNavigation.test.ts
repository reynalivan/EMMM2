import { describe, expect, it } from 'vitest';
import {
  retainPendingBrowserNavigation,
  takePendingBrowserNavigation,
  type BrowserNavigationUpdate,
} from './browserNavigation';

describe('pending browser navigation', () => {
  it('retains a document title emitted before the native tab is published', () => {
    const pending = new Map<string, BrowserNavigationUpdate>();

    retainPendingBrowserNavigation(pending, 'browser-tab-1', {
      url: 'https://example.test/one',
      title: 'Example One',
    });
    retainPendingBrowserNavigation(pending, 'browser-tab-1', {
      url: 'https://example.test/two',
    });

    expect(takePendingBrowserNavigation(pending, 'browser-tab-1')).toEqual({
      url: 'https://example.test/two',
      title: 'Example One',
    });
    expect(takePendingBrowserNavigation(pending, 'browser-tab-1')).toBeUndefined();
  });
});
