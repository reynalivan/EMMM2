import { describe, expect, it } from 'vitest';
import type { CollectionPreviewMod } from '../../../types/collection';
import { getPreviewModDisplayName } from './previewModDisplayName';

function createMod(overrides: Partial<CollectionPreviewMod>): CollectionPreviewMod {
  return {
    id: 'mod-1',
    actual_name: 'Main Mod',
    folder_path: 'E:/Mods/Main Mod',
    is_safe: true,
    object_id: null,
    object_name: null,
    object_type: null,
    ...overrides,
  };
}

describe('getPreviewModDisplayName', () => {
  it('returns actual name for non-nested mods', () => {
    expect(getPreviewModDisplayName(createMod({}))).toBe('Main Mod');
  });

  it('returns Parent > Mod for nested entries', () => {
    const value = getPreviewModDisplayName(
      createMod({
        id: 'nested_abc',
        actual_name: 'Variant A',
        folder_path: 'E:/Mods/CharacterA/Variant A',
      }),
    );

    expect(value).toBe('CharacterA > Variant A');
  });

  it('falls back to actual name when parent segment cannot be derived', () => {
    const value = getPreviewModDisplayName(
      createMod({
        id: 'nested_abc',
        actual_name: 'Variant A',
        folder_path: 'Variant A',
      }),
    );

    expect(value).toBe('Variant A');
  });
});
