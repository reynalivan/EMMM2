import { describe, expect, it } from 'vitest';
import { isFolderConflictProtected } from '@/features/workspace-runtime';

describe('isFolderConflictProtected', () => {
  it('protects an exact conflict candidate and all descendants of a parent conflict', () => {
    const scopes = ['E:/Mods/Alice'];

    expect(isFolderConflictProtected('E:/Mods/Alice', scopes)).toBe(true);
    expect(isFolderConflictProtected('E:/Mods/Alice/Variant/Blue', scopes)).toBe(true);
    expect(isFolderConflictProtected('E:/Mods/Bob/Blue', scopes)).toBe(false);
  });

  it('matches Windows path casing and separators', () => {
    expect(isFolderConflictProtected('e:\\mods\\alice\\Blue', ['E:/Mods/Alice'])).toBe(true);
  });
});
