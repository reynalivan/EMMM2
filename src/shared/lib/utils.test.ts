import { describe, it, expect, vi } from 'vitest';
import { cn, getFileUrl } from './utils';
import * as tauriCore from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

describe('utils', () => {
  describe('cn', () => {
    it('should merge tailwind classes properly', () => {
      // Normal clsx behavior
      expect(cn('class1', 'class2')).toBe('class1 class2');
      // Tailwind merge behavior (resolving conflicts)
      expect(cn('p-4', 'p-8')).toBe('p-8');
      const isFalse = false as boolean;
      expect(cn('class1', isFalse ? 'class2' : '', 'class3')).toBe('class1 class3');
    });
  });

  describe('getFileUrl', () => {
    it('should use Tuaris convertFileSrc', () => {
      const result = getFileUrl('/test/path/image.png');
      expect(tauriCore.convertFileSrc).toHaveBeenCalledWith('/test/path/image.png');
      expect(result).toBe('asset:///test/path/image.png');
    });

    it.each([
      'asset://localhost/E%3A/Mods/TestMod/preview.webp',
      'file:///E:/Mods/TestMod/preview.webp',
      'data:image/webp;base64,UklGRg==',
      'blob:http://localhost/preview-image',
    ])('keeps an existing loadable URI unchanged', (path) => {
      expect(getFileUrl(path)).toBe(path);
    });

    it('converts UNC paths through the Tauri asset protocol', () => {
      const path = '\\\\server\\mods\\TestMod\\preview.webp';

      expect(getFileUrl(path)).toBe(`asset://${path}`);
      expect(tauriCore.convertFileSrc).toHaveBeenCalledWith(path);
    });
  });
});
