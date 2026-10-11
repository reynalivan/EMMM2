import { describe, expect, it } from 'vitest';
import {
  extractFileInUsePayload,
  FileLockDiagnosticError,
  formatAppError,
  isExplorerSnapshotExpired,
} from './appError';

describe('formatAppError', () => {
  it('marks only structured I/O diagnostic failures and retains their original cause', () => {
    const cause = { type: 'App', payload: { type: 'Io', payload: 'Cannot inspect resource.' } };
    const diagnostic = FileLockDiagnosticError.fromIo(cause);
    expect(diagnostic).toBeInstanceOf(FileLockDiagnosticError);
    expect(diagnostic?.cause).toBe(cause);
    expect(formatAppError(diagnostic)).toBe('Cannot inspect resource.');
    expect(
      FileLockDiagnosticError.fromIo({ type: 'Validation', payload: 'Folder changed.' }),
    ).toBeNull();
    expect(FileLockDiagnosticError.fromIo(new Error('Rollback failed.'))).toBeNull();
  });
  it('unwraps collection lock errors and preserves the recovery capability', () => {
    expect(
      extractFileInUsePayload({
        type: 'Collection',
        payload: { type: 'FileLock', payload: { path: 'C:/Mods/A', recovery_id: 'recovery' } },
      }),
    ).toEqual({ path: 'C:/Mods/A', processes: [], recoveryId: 'recovery' });
  });
  it('keeps PathBusy actionable even without identified processes', () => {
    expect(
      extractFileInUsePayload({ type: 'PathBusy', payload: { path: 'C:/Mods/Blue' } }),
    ).toEqual({ path: 'C:/Mods/Blue', processes: [] });
  });
  it('formats an expired explorer snapshot as an actionable error', () => {
    expect(formatAppError({ type: 'ExplorerSnapshotExpired' })).toBe(
      'The folder listing changed. Reload it and select the mods again.',
    );
    expect(isExplorerSnapshotExpired('{"type":"ExplorerSnapshotExpired"}')).toBe(true);
  });
});
