import { describe, expect, it } from 'vitest';
import { resolveTogglePathRewrites } from './folderMutationPayloads';

describe('resolveTogglePathRewrites', () => {
  it.each([true, false])('preserves an explicit empty rewrite list for enable=%s', (enable) => {
    const reported: [] = [];
    expect(resolveTogglePathRewrites(['E:/Mods/Blue'], reported, enable)).toBe(reported);
  });

  it('uses only reported changes in a mixed changed/no-op batch', () => {
    const reported = [{ old_path: 'E:/Mods/DISABLED Blue', new_path: 'E:/Mods/Blue' }];
    expect(resolveTogglePathRewrites(['E:/Mods/Blue', 'E:/Mods/Red'], reported, true)).toBe(
      reported,
    );
  });

  it('retains compatibility only for a missing rewrite field', () => {
    expect(resolveTogglePathRewrites(['E:/Mods/Blue'], undefined, true)).toEqual([
      { old_path: 'E:/Mods/DISABLED Blue', new_path: 'E:/Mods/Blue' },
    ]);
  });
});
