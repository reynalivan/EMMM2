import { fireEvent, render, screen, waitFor } from '../../../testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { RecoveryDialog } from './RecoveryDialog';

const resolveRecoveryTaskMock = vi.fn();
const appStartupCheckMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  commands: {
    resolveRecoveryTask: (...args: unknown[]) => resolveRecoveryTaskMock(...args),
    appStartupCheck: () => appStartupCheckMock(),
  },
}));

vi.mock('../../../stores/useToastStore', () => ({
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
            task_type: 'switch_corridor',
            status: 'PENDING',
            target_id: 'true',
            created_at: '2026-03-29 10:00:00',
            updated_at: '2026-03-29 10:00:00',
          },
        ]}
        onResolved={onResolved}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Rollback' }));

    await waitFor(() => {
      expect(resolveRecoveryTaskMock).toHaveBeenCalledWith({
        taskId: 'task-1',
        action: 'ROLLBACK',
      });
    });

    expect(onResolved).toHaveBeenCalledWith([]);
    expect(toastSuccessMock).toHaveBeenCalled();
  });
});
