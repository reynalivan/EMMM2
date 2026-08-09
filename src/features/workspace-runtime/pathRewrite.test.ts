import { describe, expect, it } from 'vitest';
import { rewriteWorkspacePathValue } from './pathRewrite';

describe('rewriteWorkspacePathValue', () => {
  const rewrites = [{ oldPath: 'E:/Mods/Raiden/SkinA', newPath: 'E:/Mods/Raiden/DISABLED SkinA' }];

  it('preserves the original separator style', () => {
    expect(rewriteWorkspacePathValue('E:\\Mods\\Raiden\\SkinA', rewrites)).toBe(
      'E:\\Mods\\Raiden\\DISABLED SkinA',
    );
    expect(rewriteWorkspacePathValue('E:/Mods/Raiden/SkinA', rewrites)).toBe(
      'E:/Mods/Raiden/DISABLED SkinA',
    );
  });

  it('matches ASCII-case-insensitively, keeping the replacement casing', () => {
    expect(rewriteWorkspacePathValue('e:/mods/raiden/skina', rewrites)).toBe(
      'E:/Mods/Raiden/DISABLED SkinA',
    );
  });

  it('rewrites children of the renamed folder', () => {
    expect(rewriteWorkspacePathValue('E:/Mods/Raiden/SkinA/mod.ini', rewrites)).toBe(
      'E:/Mods/Raiden/DISABLED SkinA/mod.ini',
    );
  });

  it('rewrites relative spellings anchored on a whole-segment suffix', () => {
    expect(rewriteWorkspacePathValue('Raiden/SkinA', rewrites)).toBe('Raiden/DISABLED SkinA');
    expect(rewriteWorkspacePathValue('Raiden/SkinA/nested', rewrites)).toBe(
      'Raiden/DISABLED SkinA/nested',
    );
  });

  it('never touches a same-named mod under another object', () => {
    expect(rewriteWorkspacePathValue('E:/Mods/Nahida/SkinA', rewrites)).toBe(
      'E:/Mods/Nahida/SkinA',
    );
    expect(rewriteWorkspacePathValue('Nahida/SkinA', rewrites)).toBe('Nahida/SkinA');
  });
});
