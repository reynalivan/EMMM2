import { describe, it, expect } from 'vitest';
import {
  isArchivePath,
  isImagePath,
  isIniPath,
  classifyDroppedPaths,
  hasUnsupported,
  allUnsupported,
  hasArchives,
  onlyArchives,
  supportedCount,
  validateDropForZone,
} from './dropUtils';

describe('dropUtils', () => {
  describe('isArchivePath', () => {
    it('returns true for supported archives', () => {
      expect(isArchivePath('C:\\test.zip')).toBe(true);
      expect(isArchivePath('/home/test.7z')).toBe(true);
      expect(isArchivePath('mod.RAR')).toBe(true); // Case-insensitive
    });

    it('returns false for others', () => {
      expect(isArchivePath('test.txt')).toBe(false);
      expect(isArchivePath('test')).toBe(false);
    });
  });

  describe('isImagePath', () => {
    it('returns true for images', () => {
      expect(isImagePath('thumb.png')).toBe(true);
      expect(isImagePath('thumb.JPG')).toBe(true);
      expect(isImagePath('thumb.jpeg')).toBe(true);
      expect(isImagePath('thumb.webp')).toBe(true);
    });

    it('returns false for non-images', () => {
      expect(isImagePath('thumb.ini')).toBe(false);
    });
  });

  describe('isIniPath', () => {
    it('returns true for ini files', () => {
      expect(isIniPath('config.ini')).toBe(true);
      expect(isIniPath('test.INI')).toBe(true);
    });
    it('returns false for non-ini files', () => {
      expect(isIniPath('test.txt')).toBe(false);
    });
  });

  describe('classifyDroppedPaths', () => {
    it('classifies mixed paths correctly', () => {
      const paths = [
        'C:\\folder\\mod', // folder
        'C:\\folder\\mod2', // folder
        'test.zip', // archive
        'config.ini', // ini
        'preview.png', // image
        'readme.txt', // unsupported
      ];

      const result = classifyDroppedPaths(paths);
      expect(result.folders).toEqual(['C:\\folder\\mod', 'C:\\folder\\mod2']);
      expect(result.archives).toEqual(['test.zip']);
      expect(result.iniFiles).toEqual(['config.ini']);
      expect(result.images).toEqual(['preview.png']);
      expect(result.unsupported).toEqual(['readme.txt']);
    });
  });

  describe('Utility functions', () => {
    it('hasUnsupported returns true if unsupported exist', () => {
      const paths = ['readme.txt', 'mod.zip'];
      expect(hasUnsupported(classifyDroppedPaths(paths))).toBe(true);
      expect(hasUnsupported(classifyDroppedPaths(['mod.zip']))).toBe(false);
    });

    it('allUnsupported returns true if everything is unsupported', () => {
      expect(allUnsupported(classifyDroppedPaths(['readme.txt']))).toBe(true);
      expect(allUnsupported(classifyDroppedPaths(['readme.txt', 'mod']))).toBe(false);
    });

    it('hasArchives returns true if archives exist', () => {
      expect(hasArchives(classifyDroppedPaths(['mod.zip', 'folder']))).toBe(true);
      expect(hasArchives(classifyDroppedPaths(['folder']))).toBe(false);
    });

    it('onlyArchives returns true if only archives exist', () => {
      expect(onlyArchives(classifyDroppedPaths(['mod.zip', 'other.7z']))).toBe(true);
      expect(onlyArchives(classifyDroppedPaths(['mod.zip', 'folder']))).toBe(false);
    });

    it('supportedCount tallies all supported types', () => {
      const cls = classifyDroppedPaths(['folder', 'mod.zip', 'pic.png', 'cfg.ini', 'bad.txt']);
      expect(supportedCount(cls)).toBe(4);
    });
  });

  describe('validateDropForZone', () => {
    it('blocks if any unsupported file exists and all are unsupported', () => {
      const paths = ['readme.txt'];
      const classified = classifyDroppedPaths(paths);
      const res = validateDropForZone('item', classified);
      expect(res.valid).toBe(false);
      expect(res.reason).toBe('Unsupported file type');
    });

    it('blocks archives on new-object zone', () => {
      const paths = ['mod.zip'];
      const classified = classifyDroppedPaths(paths);
      const res = validateDropForZone('new-object', classified);
      expect(res.valid).toBe(false);
      expect(res.reason).toContain('Archives cannot be added');
    });

    it('allows valid combos', () => {
      const paths = ['mod_folder', 'preview.png'];
      const classified = classifyDroppedPaths(paths);
      expect(validateDropForZone('item', classified).valid).toBe(true);
      expect(validateDropForZone('new-object', classified).valid).toBe(true);
      expect(validateDropForZone('auto-organize', classified).valid).toBe(true);
    });
  });
});
