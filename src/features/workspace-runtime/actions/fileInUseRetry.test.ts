import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  openBulkFileInUseRetryDialog,
  openBulkFileLockErrorDialog,
  openFileInUseRetryDialog,
  runWithFileLockRecovery,
  FileLockRecoveryCancelled,
} from './fileInUseRetry';

const mocks = vi.hoisted(() => ({ open: vi.fn(), activeGameId: 'game' as string | null }));
const dismiss = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const getInfo = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { dismissFileLockRecovery: dismiss, getFileLockInfo: getInfo },
}));
vi.mock('../state/workspaceDialogs', () => ({ openWorkspaceFileInUseDialog: mocks.open }));
vi.mock('@/app/store', () => ({
  useAppStore: { getState: () => ({ activeGameId: mocks.activeGameId }) },
}));
vi.mock('@/shared/lib/i18n', () => ({ default: { t: (key: string) => key } }));

const lockError = (path: string, id: string) => ({
  type: 'FileLock',
  payload: { path, recovery_id: id },
});

describe('file lock replay ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.activeGameId = 'game';
    getInfo.mockImplementation(async (id: string) => ({ recovery_id: id }));
  });

  it('passes the latest capability and suppresses duplicate dialogs during awaited retry', async () => {
    const initial = { gameId: 'game', path: 'C:/Mods/A' };
    const retry = vi.fn(async (variables) => {
      expect(openFileInUseRetryDialog(lockError('C:/Mods/A', 'new-token'), variables, retry)).toBe(
        true,
      );
      throw lockError('C:/Mods/A', 'new-token');
    });
    openFileInUseRetryDialog(lockError('C:/Mods/A', 'old-token'), initial, retry);
    await expect(mocks.open.mock.calls[0][0].onRetry('latest-token')).rejects.toEqual(
      lockError('C:/Mods/A', 'new-token'),
    );
    expect(retry).toHaveBeenCalledWith({ ...initial, recoveryId: 'latest-token' });
    expect(mocks.open).toHaveBeenCalledTimes(1);
  });

  it('prevents mutation when the active workspace changed', async () => {
    const retry = vi.fn();
    openFileInUseRetryDialog(lockError('C:/Mods/A', 'token'), { path: 'C:/Mods/A' }, retry);
    mocks.activeGameId = 'other-game';
    await expect(mocks.open.mock.calls[0][0].onRetry('token')).rejects.toThrow('context_changed');
    expect(retry).not.toHaveBeenCalled();
  });

  it('retries only locked failures, then removes successful targets from later retries', async () => {
    const variables = { paths: ['C:/Mods/A', 'C:/Mods/B', 'C:/Mods/C'], gameId: 'game' };
    const retry = vi
      .fn()
      .mockResolvedValueOnce({
        success: ['C:/Mods/B'],
        failures: [{ path: 'C:/Mods/C', error: lockError('C:/Mods/C', 'new-C') }],
      })
      .mockResolvedValueOnce({ success: ['C:/Mods/C'], failures: [] });
    openBulkFileInUseRetryDialog(
      {
        success: ['C:/Mods/A'],
        failures: [
          { path: 'C:/Mods/B', error: lockError('C:/Mods/B', 'B') },
          { path: 'C:/Mods/C', error: lockError('C:/Mods/C', 'C') },
        ],
      },
      variables,
      retry,
    );
    const onRetry = mocks.open.mock.calls[0][0].onRetry;
    await expect(onRetry('B')).rejects.toEqual(lockError('C:/Mods/C', 'new-C'));
    expect(retry).toHaveBeenNthCalledWith(1, {
      ...variables,
      paths: ['C:/Mods/B', 'C:/Mods/C'],
      recoveryIds: ['B', 'C'],
    });
    expect(getInfo.mock.calls).toEqual([['B'], ['C']]);
    await onRetry('new-C');
    expect(retry).toHaveBeenNthCalledWith(2, {
      ...variables,
      paths: ['C:/Mods/C'],
      recoveryIds: ['new-C'],
    });
    expect(getInfo.mock.calls).toEqual([['B'], ['C'], ['new-C']]);
    await mocks.open.mock.calls[0][0].onDismiss();
    expect(dismiss).toHaveBeenCalledWith('B');
    expect(dismiss).toHaveBeenCalledWith('C');
    expect(dismiss).toHaveBeenCalledWith('new-C');
  });

  it('does not replay any bulk target when renewal of a pending identity fails', async () => {
    getInfo.mockRejectedValueOnce(new Error('The original folder changed.'));
    const retry = vi.fn();
    openBulkFileInUseRetryDialog(
      {
        success: ['C:/Mods/A'],
        failures: [{ path: 'C:/Mods/B', error: lockError('C:/Mods/B', 'B') }],
      },
      { paths: ['C:/Mods/A', 'C:/Mods/B'], gameId: 'game' },
      retry,
    );
    await expect(mocks.open.mock.calls[0][0].onRetry('B')).rejects.toThrow(
      'The original folder changed.',
    );
    expect(retry).not.toHaveBeenCalled();
  });

  it('replays the full rolled-back scope of a thrown transactional failure', async () => {
    const retry = vi.fn().mockResolvedValue({ success: ['C:/Mods/A', 'C:/Mods/B'], failures: [] });
    const variables = { paths: ['C:/Mods/A', 'C:/Mods/B'], gameId: 'game' };
    expect(
      openBulkFileLockErrorDialog(lockError('C:/Mods/B', 'rollback-scope'), variables, retry),
    ).toBe(true);
    await mocks.open.mock.calls[0][0].onRetry('renewed-scope');
    expect(getInfo).toHaveBeenCalledWith('renewed-scope');
    expect(retry).toHaveBeenCalledWith({ ...variables, recoveryIds: ['renewed-scope'] });
  });

  it('pauses a sequence on the locked step without replaying committed predecessors', async () => {
    const first = vi.fn().mockResolvedValue(undefined);
    const blocked = vi
      .fn()
      .mockRejectedValueOnce(lockError('C:/Mods/B', 'B'))
      .mockRejectedValueOnce(lockError('C:/Mods/B', 'B2'))
      .mockResolvedValue('done');
    const last = vi.fn().mockResolvedValue(undefined);
    const sequence = (async () => {
      await first();
      await runWithFileLockRecovery('game', blocked);
      await last();
    })();
    await vi.waitFor(() => expect(mocks.open).toHaveBeenCalledTimes(1));
    const dialog = mocks.open.mock.calls[0][0];
    await expect(dialog.onRetry('B')).rejects.toEqual(lockError('C:/Mods/B', 'B2'));
    expect(first).toHaveBeenCalledTimes(1);
    expect(last).not.toHaveBeenCalled();
    await dialog.onRetry('B2');
    await sequence;
    expect(first).toHaveBeenCalledTimes(1);
    expect(last).toHaveBeenCalledTimes(1);
    expect(blocked.mock.calls).toEqual([[null], ['B'], ['B2']]);
  });

  it('cancels a paused sequence, revokes capabilities, and prevents following commands', async () => {
    const blocked = vi.fn().mockRejectedValue(lockError('C:/Mods/B', 'B'));
    const last = vi.fn();
    const sequence = (async () => {
      await runWithFileLockRecovery('game', blocked);
      await last();
    })().catch((error: unknown) => error);
    await vi.waitFor(() => expect(mocks.open).toHaveBeenCalledTimes(1));
    const dialog = mocks.open.mock.calls[0][0];
    await dialog.onDismiss();
    expect(await sequence).toBeInstanceOf(FileLockRecoveryCancelled);
    expect(dismiss).toHaveBeenCalledWith('B');
    expect(last).not.toHaveBeenCalled();
    await expect(dialog.onRetry('B')).rejects.toBeInstanceOf(FileLockRecoveryCancelled);
    expect(blocked).toHaveBeenCalledTimes(1);
  });
});
