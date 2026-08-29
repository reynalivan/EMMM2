import { describe, it, expect } from 'vitest';
import { formatKeybindValue, parseKeybindValue } from './keybindValue';

describe('parseKeybindValue', () => {
  it('reads a plain key', () => {
    expect(parseKeybindValue('F5')).toEqual({
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      noCtrl: false,
      noAlt: false,
      noShift: false,
      mainKey: 'F5',
    });
  });

  it('reads active modifiers', () => {
    const parsed = parseKeybindValue('ctrl shift k');
    expect(parsed.ctrlKey).toBe(true);
    expect(parsed.shiftKey).toBe(true);
    expect(parsed.altKey).toBe(false);
    expect(parsed.mainKey).toBe('K');
  });

  it('treats no_<modifier> as a restriction, not an active modifier', () => {
    const parsed = parseKeybindValue('no_ctrl no_alt VK_F1');
    expect(parsed.ctrlKey).toBe(false);
    expect(parsed.altKey).toBe(false);
    expect(parsed.noCtrl).toBe(true);
    expect(parsed.noAlt).toBe(true);
    expect(parsed.noShift).toBe(false);
    expect(parsed.mainKey).toBe('VK_F1');
  });

  it('keeps the last non-modifier token as the main key', () => {
    expect(parseKeybindValue('ctrl no_shift alt X').mainKey).toBe('X');
  });

  it('yields an empty main key for an empty value', () => {
    expect(parseKeybindValue('').mainKey).toBe('');
  });
});

describe('formatKeybindValue', () => {
  const none = {
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    noCtrl: false,
    noAlt: false,
    noShift: false,
  };

  it('emits the bare key when nothing is set', () => {
    expect(formatKeybindValue(none, 'F5')).toBe('F5');
  });

  it('emits modifiers in ctrl/alt/shift order', () => {
    expect(formatKeybindValue({ ...none, ctrlKey: true, altKey: true, shiftKey: true }, 'K')).toBe(
      'ctrl alt shift K',
    );
  });

  it('drops a no_<modifier> restriction when the modifier itself is active', () => {
    expect(formatKeybindValue({ ...none, ctrlKey: true, noCtrl: true }, 'K')).toBe('ctrl K');
  });

  it('emits restrictions for inactive modifiers', () => {
    expect(formatKeybindValue({ ...none, noCtrl: true, noAlt: true, noShift: true }, 'K')).toBe(
      'no_ctrl no_alt no_shift K',
    );
  });

  it('round-trips a parsed value', () => {
    const parsed = parseKeybindValue('ctrl no_shift VK_F2');
    expect(formatKeybindValue(parsed, parsed.mainKey ?? '')).toBe('ctrl no_shift VK_F2');
  });
});
