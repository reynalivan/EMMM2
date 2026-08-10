import { describe, it, expect } from 'vitest';
import { isDisabledName, stripDisabledPrefix, toggleDisabledInPath } from './disabledPrefix';

describe('disabledPrefix', () => {
  describe('isDisabledName', () => {
    it('should identify canonical prefix', () => {
      expect(isDisabledName('DISABLED MyMod')).toBe(true);
    });

    it('should identify case variants', () => {
      expect(isDisabledName('disabled MyMod')).toBe(true);
      expect(isDisabledName('DiSaBLeD MyMod')).toBe(true);
      expect(isDisabledName('DISABLED_MyMod')).toBe(true);
      expect(isDisabledName('DISABLEDMyMod')).toBe(true);
    });

    it('should reject normal names', () => {
      expect(isDisabledName('MyMod')).toBe(false);
      expect(isDisabledName('NotDisabled')).toBe(false);
      expect(isDisabledName('distance_mod')).toBe(false);
      expect(isDisabledName('disable-MyMod')).toBe(false);
      expect(isDisabledName('Some_disable_mod')).toBe(false);
    });
  });

  describe('stripDisabledPrefix', () => {
    it('should strip canonical prefix', () => {
      expect(stripDisabledPrefix('DISABLED MyMod')).toBe('MyMod');
    });

    it('should strip canonical prefix case-insensitively', () => {
      expect(stripDisabledPrefix('disabled MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('DiSaBlEd   MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('DISABLED_MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('DISABLEDMyMod')).toBe('MyMod');
    });

    it('should leave normal names alone', () => {
      expect(stripDisabledPrefix('MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('distance_mod')).toBe('distance_mod');
    });
  });

  describe('toggleDisabledInPath', () => {
    it('should disable a previously enabled path', () => {
      expect(toggleDisabledInPath('mods/Character/MyMod', false)).toBe(
        'mods/Character/DISABLED MyMod',
      );
    });

    it('should enable a previously disabled path', () => {
      expect(toggleDisabledInPath('mods/Character/DISABLED MyMod', true)).toBe(
        'mods/Character/MyMod',
      );
    });

    it('should not double-disable a path', () => {
      expect(toggleDisabledInPath('mods/Character/DISABLED MyMod', false)).toBe(
        'mods/Character/DISABLED MyMod',
      );
    });

    it('should canonicalize a legacy runtime-disabled path when enabling', () => {
      expect(toggleDisabledInPath('mods/Character/DISABLED_MyMod', true)).toBe(
        'mods/Character/MyMod',
      );
      expect(toggleDisabledInPath('mods/Character/DISABLEDMyMod', true)).toBe(
        'mods/Character/MyMod',
      );
    });

    it('should handle windows paths implicitly if split by / or \\', () => {
      expect(toggleDisabledInPath('mods\\Character\\MyMod', false)).toBe(
        'mods/Character/DISABLED MyMod',
      );
    });
  });
});
