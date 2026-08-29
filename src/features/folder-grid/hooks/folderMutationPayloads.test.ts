import { describe, expect, it } from 'vitest';
import { resolveTogglePathRewrites } from './folderMutationPayloads';

describe('resolveTogglePathRewrites', () => {
  it('prefers the rewrites reported by the backend', () => {
    const reported = [{ old_path: 'a', new_path: 'b' }];

    expect(resolveTogglePathRewrites(['ignored'], reported, true)).toBe(reported);
  });

  it('reconstructs enable rewrites from the disabled source name', () => {
    expect(resolveTogglePathRewrites(['C:/Mods/Ayaka'], [], true)).toEqual([
      { old_path: 'C:/Mods/DISABLED Ayaka', new_path: 'C:/Mods/Ayaka' },
    ]);
  });

  it('reconstructs disable rewrites from the enabled source name', () => {
    expect(resolveTogglePathRewrites(['C:/Mods/DISABLED Ayaka'], null, false)).toEqual([
      { old_path: 'C:/Mods/Ayaka', new_path: 'C:/Mods/DISABLED Ayaka' },
    ]);
  });
});
