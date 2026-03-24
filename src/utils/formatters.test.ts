import { describe, it, expect } from 'vitest';
import { formatBytes, formatSize } from './formatters';

describe('formatters', () => {
  describe('formatBytes', () => {
    it('should format 0 bytes correctly', () => {
      expect(formatBytes(0)).toBe('0 B');
      expect(formatBytes(-100)).toBe('0 B');
    });

    it('should format bytes without decimals', () => {
      expect(formatBytes(500)).toBe('500 B');
    });

    it('should format KB correctly', () => {
      expect(formatBytes(1024)).toBe('1 KB');
      expect(formatBytes(1536)).toBe('1.5 KB');
    });

    it('should format MB correctly', () => {
      expect(formatBytes(1048576)).toBe('1 MB');
      expect(formatBytes(1048576 * 2.5)).toBe('2.5 MB');
    });

    it('should format GB correctly', () => {
      expect(formatBytes(1073741824)).toBe('1 GB');
      expect(formatBytes(1073741824 * 5.75)).toBe('5.8 GB'); // Rounded to 1 decimal
    });

    it('should support custom decimals', () => {
      expect(formatBytes(1024 * 1.234, 2)).toBe('1.23 KB');
    });

    it('should handle very large numbers', () => {
      expect(formatBytes(1024 ** 4)).toBe('1 TB');
      expect(formatBytes(1024 ** 5)).toBe('1 PB');
    });
  });

  describe('formatSize alias', () => {
    it('should be identical to formatBytes', () => {
      expect(formatSize).toBe(formatBytes);
    });
  });
});
