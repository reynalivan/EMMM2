import { describe, expect, it } from 'vitest';
import { parseModInboxTimestamp } from './time';

describe('parseModInboxTimestamp', () => {
  it('treats a naive backend timestamp as UTC', () => {
    expect(parseModInboxTimestamp('2026-08-29T10:00:00').toISOString()).toBe(
      '2026-08-29T10:00:00.000Z',
    );
  });

  it('preserves an explicit timezone', () => {
    expect(parseModInboxTimestamp('2026-08-29T10:00:00+07:00').toISOString()).toBe(
      '2026-08-29T03:00:00.000Z',
    );
  });

  it('normalizes SQLite timestamps that use a space separator', () => {
    expect(parseModInboxTimestamp('2026-08-29 10:00:00').toISOString()).toBe(
      '2026-08-29T10:00:00.000Z',
    );
  });
});
