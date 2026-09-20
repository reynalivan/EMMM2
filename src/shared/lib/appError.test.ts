import { describe, expect, it } from 'vitest';
import { formatAppError, isExplorerSnapshotExpired } from './appError';

describe('formatAppError', () => {
  it('formats an expired explorer snapshot as an actionable error', () => {
    expect(formatAppError({ type: 'ExplorerSnapshotExpired' })).toBe(
      'The folder listing changed. Reload it and select the mods again.',
    );
    expect(isExplorerSnapshotExpired('{"type":"ExplorerSnapshotExpired"}')).toBe(true);
  });
});
