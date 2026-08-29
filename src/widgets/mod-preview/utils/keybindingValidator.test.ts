import { describe, expect, it } from 'vitest';
import { isValidKeyToken, validateKeyBinding } from './keybindingValidator';

describe('keybindingValidator', () => {
  describe('isValidKeyToken', () => {
    it('accepts single printable characters', () => {
      expect(isValidKeyToken('v')).toBe(true);
      expect(isValidKeyToken('V')).toBe(true);
      expect(isValidKeyToken('1')).toBe(true);
      expect(isValidKeyToken('~')).toBe(true);
      expect(isValidKeyToken('!')).toBe(true);
      expect(isValidKeyToken(' ')).toBe(true); // space
    });

    it('rejects non-printable single characters', () => {
      expect(isValidKeyToken('\n')).toBe(false);
      expect(isValidKeyToken('\r')).toBe(false);
      expect(isValidKeyToken('\t')).toBe(false);
    });

    it('accepts VK names with VK_ prefix', () => {
      expect(isValidKeyToken('VK_F1')).toBe(true);
      expect(isValidKeyToken('VK_SPACE')).toBe(true);
      expect(isValidKeyToken('VK_GAMEPAD_A')).toBe(true);
      expect(isValidKeyToken('vk_f1')).toBe(true); // case-insensitive
    });

    it('accepts VK names without prefix', () => {
      expect(isValidKeyToken('F1')).toBe(true);
      expect(isValidKeyToken('SPACE')).toBe(true);
      expect(isValidKeyToken('GAMEPAD_A')).toBe(true);
      expect(isValidKeyToken('f1')).toBe(true); // case-insensitive
      expect(isValidKeyToken('return')).toBe(true);
    });

    it('accepts valid hex codes', () => {
      expect(isValidKeyToken('0x1B')).toBe(true);
      expect(isValidKeyToken('0x1b')).toBe(true);
      expect(isValidKeyToken('0x00A1')).toBe(true);
    });

    it('rejects invalid tokens', () => {
      expect(isValidKeyToken('NOTAKEY')).toBe(false);
      expect(isValidKeyToken('VK_NOTAKEY')).toBe(false);
      expect(isValidKeyToken('F25')).toBe(false);
      expect(isValidKeyToken('0xG1')).toBe(false);
      expect(isValidKeyToken('')).toBe(false);
    });
  });

  describe('validateKeyBinding', () => {
    it('accepts valid simple tokens', () => {
      expect(validateKeyBinding('v')).toBeNull();
      expect(validateKeyBinding('F1')).toBeNull();
      expect(validateKeyBinding('VK_SPACE')).toBeNull();
      expect(validateKeyBinding('0x1B')).toBeNull();
    });

    it('accepts valid combinations with one modifier', () => {
      expect(validateKeyBinding('ctrl f')).toBeNull();
      expect(validateKeyBinding('shift VK_F1')).toBeNull();
      expect(validateKeyBinding('alt 0x1B')).toBeNull();
      expect(validateKeyBinding('no_alt v')).toBeNull();
      expect(validateKeyBinding('no_modifiers F6')).toBeNull();
    });

    it('accepts valid combinations with multiple modifiers', () => {
      expect(validateKeyBinding('ctrl alt shift VK_DELETE')).toBeNull();
      expect(validateKeyBinding('no_alt ctrl g')).toBeNull();
      expect(validateKeyBinding('shift no_alt no_shift F1')).toBeNull();
    });

    it('handles extra whitespace gracefully', () => {
      expect(validateKeyBinding('  ctrl   alt   f  ')).toBeNull();
    });

    it('rejects invalid key tokens', () => {
      expect(validateKeyBinding('INVALID_KEY_NAME')).toBe('Invalid key token: "INVALID_KEY_NAME".');
      expect(validateKeyBinding('ctrl alt INVALID_KEY_NAME')).toBe(
        'Invalid key token: "INVALID_KEY_NAME".',
      );
    });

    it('rejects missing key tokens (modifier only)', () => {
      expect(validateKeyBinding('ctrl')).toBe('Invalid key token: "ctrl".');
      expect(validateKeyBinding('alt no_modifiers')).toBe('Invalid key token: "no_modifiers".');
      expect(validateKeyBinding('no_modifiers')).toBe('Invalid key token: "no_modifiers".');
    });

    it('rejects invalid modifiers', () => {
      expect(validateKeyBinding('notamodifier f')).toBe('Invalid modifier: "notamodifier".');
      expect(validateKeyBinding('ctrl notamodifier f')).toBe('Invalid modifier: "notamodifier".');
    });

    it('rejects duplicate modifiers', () => {
      expect(validateKeyBinding('ctrl ctrl f')).toBe('Duplicate modifier: "ctrl".');
      expect(validateKeyBinding('ALT ctrl alt f')).toBe('Duplicate modifier: "alt".');
    });

    it('rejects empty strings', () => {
      expect(validateKeyBinding('')).toBe('Keybinding cannot be empty.');
      expect(validateKeyBinding('   ')).toBe('Keybinding cannot be empty.');
    });
  });
});
