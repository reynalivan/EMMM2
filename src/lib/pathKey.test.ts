import { describe, expect, it } from 'vitest';
import { canonicalPathKey, pathStartsWith, pathsEqual, relativePathFromRoot } from './pathKey';

describe('pathKey', () => {
  it('canonicalizes path with ASCII-only case folding and preserves unicode', () => {
    expect(canonicalPathKey('E:\\Mods\\한국Character\\日本語MOD\\')).toBe(
      'e:/mods/한국character/日本語mod',
    );
  });

  it('treats unicode paths as equal across slash and ASCII case changes', () => {
    expect(
      pathsEqual('E:\\Mods\\한국Character\\日本語MOD', 'e:/mods/한국character/日本語mod/'),
    ).toBe(true);
  });

  it('detects child path relationship without lowercasing unicode text', () => {
    expect(
      pathStartsWith('E:/Mods/한국Character', 'e:\\mods\\한국character\\日本語MOD\\Assets'),
    ).toBe(true);
    expect(
      relativePathFromRoot('E:/Mods/한국Character', 'e:\\mods\\한국character\\日本語MOD\\Assets'),
    ).toBe('日本語mod/assets');
  });
});
