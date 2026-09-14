import { describe, expect, it } from 'vitest';
import { resolveTogglePathRewrites } from '@/features/mod-runtime';

describe('resolveTogglePathRewrites', () => {
  it('prefers the rewrites reported by the backend', () => {
    const reported = [{ old_path: 'a', new_path: 'b' }];

    expect(resolveTogglePathRewrites(['ignored'], reported, true)).toBe(reported);
  });

  it('keeps an empty backend result as no rewrite', () => {
    expect(resolveTogglePathRewrites(['C:/Mods/Ayaka'], [], true)).toEqual([]);
  });

  it('reconstructs enable rewrites from the disabled source name', () => {
    expect(resolveTogglePathRewrites(['C:/Mods/Ayaka'], null, true)).toEqual([
      { old_path: 'C:/Mods/DISABLED Ayaka', new_path: 'C:/Mods/Ayaka' },
    ]);
  });

  it('reconstructs disable rewrites from the enabled source name', () => {
    expect(resolveTogglePathRewrites(['C:/Mods/DISABLED Ayaka'], null, false)).toEqual([
      { old_path: 'C:/Mods/Ayaka', new_path: 'C:/Mods/DISABLED Ayaka' },
    ]);
  });
});
