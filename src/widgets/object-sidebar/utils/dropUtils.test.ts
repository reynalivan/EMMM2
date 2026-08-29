import { describe, it, expect } from 'vitest';
import { classifyDroppedPaths, allUnsupported, validateDropForZone } from './dropUtils';

describe('dropUtils', () => {
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
    it('allUnsupported returns true if everything is unsupported', () => {
      expect(allUnsupported(classifyDroppedPaths(['readme.txt']))).toBe(true);
      expect(allUnsupported(classifyDroppedPaths(['readme.txt', 'mod']))).toBe(false);
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
