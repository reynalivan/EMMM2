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
});
