import { describe, expect, it } from 'vitest';
import { normalizeBrowserUrl, tabDisplayLabel } from './browserUrl';

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

  it('returns null when there is nothing to show', () => {
    expect(tabDisplayLabel({ id: 'a', url: '', title: '' })).toBeNull();
  });
});
