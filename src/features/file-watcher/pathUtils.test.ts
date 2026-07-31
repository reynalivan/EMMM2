import { describe, expect, it } from 'vitest';
import { normalizeWorkspacePath } from '../workspace-runtime/pathRewrite';
import { joinModPath, rewritePath } from './pathUtils';

describe('workspace path normalization', () => {
  it('strips trailing slashes so prefix matching stays sound', () => {
    expect(normalizeWorkspacePath('E:\\Mods\\')).toBe('E:/Mods');
    expect(normalizeWorkspacePath('E:/Mods//')).toBe('E:/Mods');
    expect(normalizeWorkspacePath('E:/Mods')).toBe('E:/Mods');
  });

  it('joins a mods root that carries a trailing slash without doubling it', () => {
    expect(joinModPath('E:\\Mods\\', '/Nahida')).toBe('E:/Mods/Nahida');
    expect(joinModPath('E:/Mods', 'Nahida')).toBe('E:/Mods/Nahida');
    expect(joinModPath('E:/Mods/', '')).toBe('E:/Mods');
  });

  it('rewrites descendants when the source path carries a trailing slash', () => {
    expect(rewritePath('E:/Mods/Nahida/skin.ini', 'E:/Mods/Nahida/', 'E:/Mods/Raiden')).toBe(
      'E:/Mods/Raiden/skin.ini',
    );
    expect(rewritePath('E:\\Mods\\Nahida\\', 'E:/Mods/Nahida', 'E:/Mods/Raiden')).toBe(
      'E:/Mods/Raiden',
    );
    expect(rewritePath('E:/Mods/Other', 'E:/Mods/Nahida/', 'E:/Mods/Raiden')).toBeNull();
  });
});
