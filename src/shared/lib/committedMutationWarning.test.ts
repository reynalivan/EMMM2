import { beforeEach, describe, expect, it, vi } from 'vitest';

const warningToast = vi.fn();

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    warning: (...args: unknown[]) => warningToast(...args),
  },
}));

import { notifyCommittedMutationSyncWarning } from './committedMutationWarning';

describe('notifyCommittedMutationSyncWarning', () => {
  beforeEach(() => {
    warningToast.mockReset();
  });

  it('presents a non-fatal warning after a committed mutation', () => {
    notifyCommittedMutationSyncWarning({
      sync_warning: {
        kind: 'ReconcileFailed',
        message: 'projection retry is pending',
      },
    });

    expect(warningToast).toHaveBeenCalledWith(
      'Disk changes were applied, but runtime refresh is still pending.',
      7000,
    );
  });

  it('does nothing when terminal reconcile completed', () => {
    notifyCommittedMutationSyncWarning({ sync_warning: null });

    expect(warningToast).not.toHaveBeenCalled();
  });

  it('names the required manual reload action', () => {
    notifyCommittedMutationSyncWarning({
      sync_warning: {
        kind: 'ManualReloadRequired',
        message:
          'Manual reload required: focus the active game and press F10 to reload its configuration.',
      },
    });

    expect(warningToast).toHaveBeenCalledWith(
      'Manual reload required: focus the game, then press F10.',
      7000,
    );
  });
});
