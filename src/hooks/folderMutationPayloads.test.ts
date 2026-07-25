import { describe, expect, it } from 'vitest';
import type { ModFolder } from '../types/mod';
import { applyModInfoUpdate, resolveTogglePathRewrites } from './folderMutationPayloads';

const folder = {
  path: 'C:/Mods/Ayaka',
  name: 'Ayaka',
  is_favorite: true,
  is_safe: false,
  metadata: { author: 'someone', version: '1.0' },
} as unknown as ModFolder;

describe('applyModInfoUpdate', () => {
  it('keeps the current values when the update omits them', () => {
    const next = applyModInfoUpdate(folder, {});

    expect(next.is_favorite).toBe(true);
    expect(next.is_safe).toBe(false);
    expect(next.metadata).toBe(folder.metadata);
  });

  it('applies false explicitly instead of treating it as absent', () => {
    expect(applyModInfoUpdate(folder, { is_favorite: false }).is_favorite).toBe(false);
  });

  it('merges metadata over the existing fields', () => {
    const next = applyModInfoUpdate(folder, { metadata: { version: '2.0' } });

    expect(next.metadata).toEqual({ author: 'someone', version: '2.0' });
    expect(folder.metadata).toEqual({ author: 'someone', version: '1.0' });
  });
});

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
