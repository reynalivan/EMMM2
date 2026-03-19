import { describe, expect, it } from 'vitest';
import { isModFolder } from './hooks';

describe('isModFolder', () => {
  it('accepts unicode mod folder path with slash and ASCII case variants', () => {
    expect(
      isModFolder(
        'e:\\mods\\한국character\\日本語mod',
        'E:/Mods',
      ),
    ).toBe(true);
  });

  it('rejects unicode file path under mod root', () => {
    expect(
      isModFolder(
        'e:\\mods\\한국character\\日本語mod\\config.ini',
        'E:/Mods',
      ),
    ).toBe(false);
  });
});
