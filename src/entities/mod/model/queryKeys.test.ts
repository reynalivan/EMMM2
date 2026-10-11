import { describe, expect, it } from 'vitest';
import { thumbnailKeys } from './queryKeys';

describe('thumbnailKeys', () => {
  it('keeps equal relative folder paths separate between games', () => {
    const folderPath = 'Character/Mod A';

    expect(thumbnailKeys.folder(folderPath, 'game-a')).not.toEqual(
      thumbnailKeys.folder(folderPath, 'game-b'),
    );
  });

  it('keeps the path-only key as an invalidation prefix', () => {
    expect(thumbnailKeys.folder('Character/Mod A')).toEqual(['thumbnails', 'character/mod a']);
  });

  it('keeps coexisting enabled and disabled physical folders separate', () => {
    expect(thumbnailKeys.folder('E:/Mods/Alice/Blue', 'game-a')).not.toEqual(
      thumbnailKeys.folder('E:/Mods/Alice/DISABLED Blue', 'game-a'),
    );
    expect(thumbnailKeys.folder('E:/Mods/Alice/Blue', 'game-a')).not.toEqual(
      thumbnailKeys.folder('E:/Mods/DISABLED Alice/Blue', 'game-a'),
    );
  });

  it('normalizes casing and separators without stripping physical folder prefixes', () => {
    expect(thumbnailKeys.folder('E:/Mods/DISABLED Alice/Blue', 'game-a')).toEqual(
      thumbnailKeys.folder('e:\\mods\\DISABLED Alice\\Blue', 'game-a'),
    );
  });
});
