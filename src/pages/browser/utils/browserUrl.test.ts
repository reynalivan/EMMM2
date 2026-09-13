import { describe, expect, it } from 'vitest';
import { browserAddressParts, normalizeBrowserUrl, tabDisplayLabel } from './browserUrl';

describe('normalizeBrowserUrl', () => {
  it('keeps http and about schemes untouched', () => {
    expect(normalizeBrowserUrl('  https://gamebanana.com  ')).toBe('https://gamebanana.com');
    expect(normalizeBrowserUrl('http://localhost:1420')).toBe('http://localhost:1420');
    expect(normalizeBrowserUrl('about:blank')).toBe('about:blank');
  });

  it('prefixes https for bare hosts', () => {
    expect(normalizeBrowserUrl('gamebanana.com')).toBe('https://gamebanana.com');
    expect(normalizeBrowserUrl(' www.google.com ')).toBe('https://www.google.com');
  });
});

describe('tabDisplayLabel', () => {
  it('prefers a real page title', () => {
    expect(tabDisplayLabel({ id: 'a', url: 'https://gamebanana.com/mods', title: 'Mods' })).toBe(
      'Mods',
    );
  });

  it('falls back to the hostname while the title is still a placeholder', () => {
    expect(
      tabDisplayLabel({ id: 'a', url: 'https://gamebanana.com/mods', title: 'Loading...' }),
    ).toBe('gamebanana.com');
    expect(tabDisplayLabel({ id: 'a', url: 'https://www.google.com/', title: '' })).toBe(
      'www.google.com',
    );
  });

  it('keeps the last known document title when a URL lifecycle event has no title', () => {
    const currentTab = { id: 'a', url: 'https://gamebanana.com/mods', title: 'Mods' };
    const lifecycleUpdate: { url: string; title?: string } = {
      url: 'https://gamebanana.com/mods/new',
    };
    const updatedTab = {
      ...currentTab,
      url: lifecycleUpdate.url,
      ...(lifecycleUpdate.title?.trim() ? { title: lifecycleUpdate.title } : {}),
    };

    expect(tabDisplayLabel(updatedTab)).toBe('Mods');
  });

  it('returns null when there is nothing to show', () => {
    expect(tabDisplayLabel({ id: 'a', url: '', title: '' })).toBeNull();
  });
});

describe('browserAddressParts', () => {
  it('gives the host priority over the path, query, and fragment', () => {
    expect(browserAddressParts('https://gamebanana.com/mods/123?sort=recent#files')).toEqual({
      host: 'gamebanana.com',
      suffix: '/mods/123?sort=recent#files',
    });
  });

  it('does not format non-web addresses as a trusted host', () => {
    expect(browserAddressParts('about:blank')).toBeNull();
    expect(browserAddressParts('not a url')).toBeNull();
  });
});
