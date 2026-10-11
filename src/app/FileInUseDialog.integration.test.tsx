import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FileInUseDialog } from '@/features/file-watcher';
import { useEffect } from 'react';
import { openBulkFileInUseRetryDialog } from '@/features/workspace-runtime';

const mocks = vi.hoisted(() => ({
  data: null as {
    path: string;
    processes: string[];
    recoveryId?: string;
    onRetry?: (recoveryId?: string) => Promise<unknown>;
  } | null,
  close: vi.fn(),
  getInfo: vi.fn(),
  control: vi.fn(),
  dismiss: vi.fn(),
}));

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock('@/app/store', () => ({
  useAppStore: { getState: () => ({ activeGameId: 'game' }) },
}));
vi.mock('@/shared/lib/i18n', () => ({ default: { t: (key: string) => key } }));
vi.mock('@/shared/i18n/config', () => ({ default: { t: (key: string) => key } }));
vi.mock('@/features/workspace-runtime/state/workspaceDialogs', () => ({
  openWorkspaceFileInUseDialog: (data: NonNullable<typeof mocks.data>) => {
    mocks.data = data;
  },
}));
vi.mock('@/shared/lib/hooks/useDialogSync', () => ({
  useDialogSync: (ref: { current: HTMLDialogElement | null }, open: boolean) => {
    useEffect(() => {
      if (ref.current) ref.current.open = open;
    }, [ref, open]);
  },
}));
vi.mock('@/features/workspace-runtime/@x/file-watcher', () => ({
  useWorkspaceRuntimeSelector: (select: (state: unknown) => unknown) =>
    select({
      dialogState: mocks.data ? { kind: 'fileInUse', data: mocks.data } : { kind: 'none' },
    }),
  getWorkspaceRuntimeState: () => ({
    dialogState: mocks.data ? { kind: 'fileInUse', data: mocks.data } : { kind: 'none' },
  }),
  closeWorkspaceDialog: mocks.close,
}));
vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    getFileLockInfo: mocks.getInfo,
    controlFileLockProcess: mocks.control,
    dismissFileLockRecovery: mocks.dismiss,
  },
}));

describe('persistent file lock recovery dialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getInfo.mockReset();
    mocks.getInfo.mockResolvedValue({
      recovery_id: 'recovery',
      path: 'C:/Mods/A',
      status: 'unknown',
      message: null,
      processes: [],
      action_outcome: null,
    });
    mocks.dismiss.mockResolvedValue(undefined);
    mocks.data = { path: 'C:/Mods/A', processes: [] };
  });

  it('remains open after repeated lock failures and closes only after success', async () => {
    const retry = vi
      .fn()
      .mockRejectedValueOnce({ type: 'PathBusy', payload: { path: 'C:/Mods/A' } })
      .mockRejectedValueOnce({ type: 'PathBusy', payload: { path: 'C:/Mods/A' } })
      .mockResolvedValue(undefined);
    mocks.data!.onRetry = retry;
    render(<FileInUseDialog />);
    for (let attempt = 1; attempt <= 2; attempt++) {
      fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
      await waitFor(() => expect(retry).toHaveBeenCalledTimes(attempt));
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'actions.retry' })).not.toBeDisabled(),
      );
      expect(mocks.close).not.toHaveBeenCalled();
    }
    fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
    await waitFor(() => expect(mocks.close).toHaveBeenCalledTimes(1));
  });

  it('does not close a newer dialog when the older retry completes', async () => {
    let finish!: () => void;
    mocks.data!.onRetry = () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      });
    render(<FileInUseDialog />);
    fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
    mocks.data = { path: 'C:/Mods/B', processes: [] };
    await act(async () => finish());
    expect(mocks.close).not.toHaveBeenCalled();
  });

  it('prevents duplicate retries while the operation is pending', async () => {
    let finish!: () => void;
    const retry = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    mocks.data!.onRetry = retry;
    render(<FileInUseDialog />);
    const button = screen.getByRole('button', { name: 'actions.retry' });
    fireEvent.click(button);
    fireEvent.click(button);
    expect(retry).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: 'actions.cancel' })).toBeDisabled();
    await act(async () => finish());
  });

  it('renews an idle recovery context before manually retrying the operation', async () => {
    const retry = vi.fn().mockResolvedValue(undefined);
    mocks.data = {
      path: 'C:/Mods/A',
      processes: [],
      recoveryId: 'retained-context',
      onRetry: retry,
    };
    mocks.getInfo
      .mockResolvedValueOnce({
        recovery_id: 'retained-context',
        path: 'C:/Mods/A',
        status: 'unknown',
        message: null,
        processes: [],
        action_outcome: null,
      })
      .mockResolvedValueOnce({
        recovery_id: 'renewed-context',
        path: 'C:/Mods/A',
        status: 'unknown',
        message: null,
        processes: [],
        action_outcome: null,
      });
    render(<FileInUseDialog />);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'actions.retry' })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
    await waitFor(() => expect(retry).toHaveBeenCalledWith('renewed-context'));
    expect(mocks.getInfo).toHaveBeenNthCalledWith(2, 'retained-context');
    expect(mocks.getInfo.mock.invocationCallOrder[1]).toBeLessThan(
      retry.mock.invocationCallOrder[0],
    );
  });

  it('keeps a rejected context renewal visible without replaying the operation', async () => {
    const retry = vi.fn();
    mocks.data = { path: 'C:/Mods/A', processes: [], recoveryId: 'context', onRetry: retry };
    mocks.getInfo
      .mockResolvedValueOnce({
        recovery_id: 'context',
        path: 'C:/Mods/A',
        status: 'unknown',
        message: null,
        processes: [],
        action_outcome: null,
      })
      .mockRejectedValueOnce(new Error('The original folder changed.'));
    render(<FileInUseDialog />);
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'actions.retry' })).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
    await screen.findByText('The original folder changed.');
    expect(retry).not.toHaveBeenCalled();
    expect(mocks.close).not.toHaveBeenCalled();
  });

  it('allows retry after a transient diagnostic failure and a successful refresh', async () => {
    const retry = vi.fn().mockResolvedValue(undefined);
    mocks.data = { path: 'C:/Mods/A', processes: [], recoveryId: 'context', onRetry: retry };
    render(<FileInUseDialog />);
    await waitFor(() => expect(mocks.getInfo).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled());

    mocks.getInfo.mockRejectedValueOnce({
      type: 'Io',
      payload: 'Diagnostic temporarily unavailable.',
    });
    fireEvent.click(screen.getByTestId('file-lock-retry'));
    await screen.findByText('Diagnostic temporarily unavailable.');
    expect(retry).not.toHaveBeenCalled();
    expect(mocks.close).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('file-lock-refresh'));
    await waitFor(() => expect(mocks.getInfo).toHaveBeenCalledTimes(3));
    await waitFor(() => expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled());
    fireEvent.click(screen.getByTestId('file-lock-retry'));
    await waitFor(() => expect(retry).toHaveBeenCalledWith('recovery'));
    await waitFor(() => expect(mocks.close).toHaveBeenCalledTimes(1));
  });

  it('does not re-enable unsafe replay after diagnostics refresh successfully', async () => {
    const retry = vi.fn().mockRejectedValue({
      type: 'Validation',
      payload: 'Partial rollback failed. Repair the workspace.',
    });
    mocks.data = { path: 'C:/Mods/A', processes: [], recoveryId: 'context', onRetry: retry };
    render(<FileInUseDialog />);
    await waitFor(() => expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled());
    fireEvent.click(screen.getByTestId('file-lock-retry'));
    await screen.findByText('Partial rollback failed. Repair the workspace.');
    expect(screen.getByTestId('file-lock-retry')).toBeDisabled();

    fireEvent.click(screen.getByTestId('file-lock-refresh'));
    await waitFor(() => expect(mocks.getInfo).toHaveBeenCalledTimes(3));
    await waitFor(() => expect(screen.getByTestId('file-lock-refresh')).not.toBeDisabled());
    expect(screen.getByTestId('file-lock-retry')).toBeDisabled();
    expect(retry).toHaveBeenCalledTimes(1);
    expect(mocks.close).not.toHaveBeenCalled();
  });

  it('keeps composed bulk retry available when a pending diagnostic fails before mutation', async () => {
    const retry = vi.fn().mockResolvedValue({ success: ['C:/Mods/A'], failures: [] });
    openBulkFileInUseRetryDialog(
      {
        success: [],
        failures: [
          {
            path: 'C:/Mods/A',
            error: { type: 'FileLock', payload: { path: 'C:/Mods/A', recovery_id: 'context' } },
          },
        ],
      },
      { paths: ['C:/Mods/A'], gameId: 'game' },
      retry,
    );
    render(<FileInUseDialog />);
    await waitFor(() => expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled());
    mocks.getInfo
      .mockResolvedValueOnce({ recovery_id: 'renewed-context' })
      .mockRejectedValueOnce({ type: 'Io', payload: 'Nested diagnostic temporarily unavailable.' });
    fireEvent.click(screen.getByTestId('file-lock-retry'));
    await screen.findByText('Nested diagnostic temporarily unavailable.');
    expect(retry).not.toHaveBeenCalled();
    expect(mocks.close).not.toHaveBeenCalled();
    expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled();
    fireEvent.click(screen.getByTestId('file-lock-retry'));
    await waitFor(() =>
      expect(retry).toHaveBeenCalledWith({
        paths: ['C:/Mods/A'],
        gameId: 'game',
        recoveryIds: ['recovery'],
      }),
    );
    await waitFor(() => expect(mocks.close).toHaveBeenCalledTimes(1));
  });

  it.each(['stale identity', 'mutation I/O failure'])(
    'blocks composed bulk replay after %s even when refresh succeeds',
    async (failureStage) => {
      const retry = vi.fn().mockRejectedValue({ type: 'Io', payload: 'Rollback failed.' });
      openBulkFileInUseRetryDialog(
        {
          success: [],
          failures: [
            {
              path: 'C:/Mods/A',
              error: { type: 'FileLock', payload: { path: 'C:/Mods/A', recovery_id: 'context' } },
            },
          ],
        },
        { paths: ['C:/Mods/A'], gameId: 'game' },
        retry,
      );
      render(<FileInUseDialog />);
      await waitFor(() => expect(screen.getByTestId('file-lock-retry')).not.toBeDisabled());
      if (failureStage === 'stale identity') {
        mocks.getInfo
          .mockResolvedValueOnce({ recovery_id: 'renewed-context' })
          .mockRejectedValueOnce({ type: 'Validation', payload: 'The original folder changed.' });
      }
      fireEvent.click(screen.getByTestId('file-lock-retry'));
      await screen.findByText(
        failureStage === 'stale identity' ? 'The original folder changed.' : 'Rollback failed.',
      );
      expect(screen.getByTestId('file-lock-retry')).toBeDisabled();
      const callsBeforeRefresh = mocks.getInfo.mock.calls.length;
      fireEvent.click(screen.getByTestId('file-lock-refresh'));
      await waitFor(() => expect(mocks.getInfo).toHaveBeenCalledTimes(callsBeforeRefresh + 1));
      await waitFor(() => expect(screen.getByTestId('file-lock-refresh')).not.toBeDisabled());
      expect(screen.getByTestId('file-lock-retry')).toBeDisabled();
      expect(retry).toHaveBeenCalledTimes(failureStage === 'stale identity' ? 0 : 1);
      expect(mocks.close).not.toHaveBeenCalled();
    },
  );

  it('keeps a changed failure visible and prevents unsafe replay', async () => {
    mocks.data!.onRetry = vi
      .fn()
      .mockRejectedValue(new Error('Partial rollback failed. Repair the workspace.'));
    render(<FileInUseDialog />);
    fireEvent.click(screen.getByRole('button', { name: 'actions.retry' }));
    await screen.findByText('Partial rollback failed. Repair the workspace.');
    expect(screen.getByRole('button', { name: 'actions.retry' })).toBeDisabled();
    expect(mocks.close).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'actions.cancel' }));
    expect(mocks.close).toHaveBeenCalledTimes(1);
  });

  it('requires confirmation to force terminate and does not automatically retry the mutation', async () => {
    const retry = vi.fn();
    mocks.data = { path: 'C:/Mods/A', processes: [], recoveryId: 'recovery', onRetry: retry };
    const report = {
      recovery_id: 'recovery',
      path: 'C:/Mods/A',
      status: 'complete',
      message: null,
      action_outcome: null,
      processes: [
        {
          process_ref: 'process-capability',
          pid: 41,
          name: 'Editor',
          executable: 'editor.exe',
          started_at: '10',
          can_close: true,
          can_terminate: true,
          unavailable_reason: null,
        },
      ],
    };
    mocks.getInfo.mockResolvedValue(report);
    mocks.control.mockResolvedValue({ ...report, action_outcome: 'exited' });
    render(<FileInUseDialog />);
    fireEvent.click(await screen.findByRole('button', { name: 'file_in_use.force_terminate' }));
    expect(mocks.control).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'file_in_use.confirm_terminate' }));
    await waitFor(() =>
      expect(mocks.control).toHaveBeenCalledWith('recovery', 'process-capability', true),
    );
    expect(retry).not.toHaveBeenCalled();
    expect(mocks.close).not.toHaveBeenCalled();
  });
});
