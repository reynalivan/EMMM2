import { fireEvent, render, screen, waitFor } from '../../../tests/testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { RecoveryDialog } from './RecoveryDialog';

const resolveRecoveryTaskMock = vi.fn();
const appStartupCheckMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    resolveRecoveryTask: (...args: unknown[]) => resolveRecoveryTaskMock(...args),
    appStartupCheck: () => appStartupCheckMock(),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

describe('RecoveryDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resolveRecoveryTaskMock.mockResolvedValue(undefined);
    appStartupCheckMock.mockResolvedValue([]);
  });

  it('resolves rollback through the structured recovery command', async () => {
    const onResolved = vi.fn();

    render(
      <RecoveryDialog
        tasks={[
          {
            id: 'task-1',
            game_id: 'game-1',
            task_type: 'apply_collection',
            status: 'PENDING',
            target_id: 'collection-1',
            rollback_collection_id: null,
            rollback_active_collection_id: null,
            final_active_collection_id: null,
            created_at: '2026-03-29 10:00:00',
            updated_at: '2026-03-29 10:00:00',
          },
        ]}
        onResolved={onResolved}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Rollback' }));

    await waitFor(() => {
      expect(resolveRecoveryTaskMock).toHaveBeenCalledWith('task-1', 'ROLLBACK');
    });

    expect(onResolved).toHaveBeenCalledWith([]);
    expect(toastSuccessMock).toHaveBeenCalled();
  });
});
