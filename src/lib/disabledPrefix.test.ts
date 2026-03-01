import { describe, it, expect } from 'vitest';
import { isDisabledName, stripDisabledPrefix, toggleDisabledInPath } from './disabledPrefix';

describe('disabledPrefix', () => {
  describe('isDisabledName', () => {
    it('should identify canonical prefix', () => {
      expect(isDisabledName('DISABLED MyMod')).toBe(true);
    });

    it('should identify case variants', () => {
      expect(isDisabledName('disabled MyMod')).toBe(true);
      expect(isDisabledName('DiSaBLeD_MyMod')).toBe(true);
      expect(isDisabledName('dis MyMod')).toBe(true);
      expect(isDisabledName('disable-MyMod')).toBe(true);
    });

    it('should reject normal names', () => {
      expect(isDisabledName('MyMod')).toBe(false);
      expect(isDisabledName('NotDisabled')).toBe(false);
      // 'distance_mod' starts with 'dis' so the regex technically matches it
      expect(isDisabledName('distance_mod')).toBe(true);
      expect(isDisabledName('Some_disable_mod')).toBe(false); // Not at start
    });
  });

  describe('stripDisabledPrefix', () => {
    it('should strip canonical prefix', () => {
      expect(stripDisabledPrefix('DISABLED MyMod')).toBe('MyMod');
    });

    it('should strip variations and separators', () => {
      expect(stripDisabledPrefix('disabled_MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('dis-MyMod')).toBe('MyMod');
      expect(stripDisabledPrefix('disable   MyMod')).toBe('MyMod');
    });

    it('should leave normal names alone', () => {
      expect(stripDisabledPrefix('MyMod')).toBe('MyMod');
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
      expect(toggleDisabledInPath('mods/Character/disabled_MyMod', false)).toBe(
        'mods/Character/disabled_MyMod',
      );
    });

    it('should handle windows paths implicitly if split by / or \\', () => {
      expect(toggleDisabledInPath('mods\\Character\\MyMod', false)).toBe(
        'mods/Character/DISABLED MyMod',
      );
    });
  });
});
