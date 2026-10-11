import { describe, expect, it } from 'vitest';
import { assertForegroundGameIdentity, type ForegroundIdentity } from './hotkeyIdentity.js';

const expected = {
  pid: 42,
  executable: 'D:\\Games\\Genshin Impact\\GenshinImpact.exe',
  title: 'Genshin Impact',
};
const observed: ForegroundIdentity = {
  pid: 42,
  executable: expected.executable,
  title: expected.title,
  creationTime: '01dd3965e0445800',
  error: 0,
};

describe('native foreground game identity', () => {
  it('accepts observed executable, title and process birth identity', () => {
    expect(() => assertForegroundGameIdentity(expected, observed)).not.toThrow();
    expect(() =>
      assertForegroundGameIdentity(
        expected,
        { ...observed, executable: expected.executable.toUpperCase() },
        observed,
      ),
    ).not.toThrow();
  });

  it('rejects missing or inaccessible OS observations', () => {
    expect(() => assertForegroundGameIdentity(expected, undefined)).toThrow();
    expect(() => assertForegroundGameIdentity(expected, { ...observed, error: 5 })).toThrow();
  });

  it('rejects a fabricated executable, title or PID', () => {
    for (const invalid of [
      { ...observed, executable: 'D:\\Other\\GenshinImpact.exe' },
      { ...observed, title: 'Another application' },
      { ...observed, pid: 43 },
      { ...observed, creationTime: null },
    ])
      expect(() => assertForegroundGameIdentity(expected, invalid)).toThrow();
  });

  it('rejects PID reuse between before and after observations', () => {
    expect(() =>
      assertForegroundGameIdentity(
        expected,
        { ...observed, creationTime: '01dd3965e0445900' },
        observed,
      ),
    ).toThrow();
  });
});
