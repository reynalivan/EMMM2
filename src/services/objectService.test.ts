import { describe, it, expect } from 'vitest';
import { validateObjectName } from './objectService';

/**
 * Tests for validateObjectName — pure function, no mocking needed.
 * Covers: NC-3.3-03 (Invalid/Reserved Names)
 */
describe('validateObjectName', () => {
  // TC-3.3-01: Valid names
  it('accepts a normal name', () => {
    expect(validateObjectName('Raiden Shogun')).toBeNull();
  });

  it('accepts names with unicode characters', () => {
    expect(validateObjectName('雷電将軍 Raiden')).toBeNull();
  });

  it('accepts name with numbers and dashes', () => {
    expect(validateObjectName('Mod-Pack-v2')).toBeNull();
  });

  // NC-3.3-03: Too short
  it('rejects empty string', () => {
    expect(validateObjectName('')).toContain('at least 2');
  });

  it('rejects single character', () => {
    expect(validateObjectName('A')).toContain('at least 2');
  });

  it('rejects whitespace-only', () => {
    expect(validateObjectName('   ')).toContain('at least 2');
  });

  // NC-3.3-03: Too long
  it('rejects names longer than 255 characters', () => {
    const longName = 'A'.repeat(256);
    expect(validateObjectName(longName)).toContain('at most 255');
  });

  // NC-3.3-03: Reserved Windows names
  it('rejects CON as reserved', () => {
    expect(validateObjectName('CON')).toContain('reserved');
  });

  it('rejects PRN case-insensitively', () => {
    expect(validateObjectName('prn')).toContain('reserved');
  });

  it('rejects COM1', () => {
    expect(validateObjectName('COM1')).toContain('reserved');
  });

  it('rejects LPT3', () => {
    expect(validateObjectName('lpt3')).toContain('reserved');
  });

  // NC-3.3-03: Invalid characters
  it('rejects names with <', () => {
    expect(validateObjectName('Test<Mod')).toContain('invalid characters');
  });

  it('rejects names with :', () => {
    expect(validateObjectName('Test:Mod')).toContain('invalid characters');
  });

  it('rejects names with |', () => {
    expect(validateObjectName('Test|Mod')).toContain('invalid characters');
  });

  it('rejects names with *', () => {
    expect(validateObjectName('Test*Mod')).toContain('invalid characters');
  });

  it('rejects names with "', () => {
    expect(validateObjectName('Test"Mod')).toContain('invalid characters');
  });

  // NC-3.3-03: Path traversal
  it('rejects dot-only names', () => {
    expect(validateObjectName('..')).toContain('cannot be only dots');
  });

  it('rejects triple dots', () => {
    expect(validateObjectName('...')).toContain('cannot be only dots');
  });

  it('rejects path traversal in name', () => {
    expect(validateObjectName('test..path')).toContain('path traversal');
  });

  // Edge: trims whitespace
  it('trims whitespace before validation', () => {
    expect(validateObjectName('  Raiden  ')).toBeNull();
  });
});
