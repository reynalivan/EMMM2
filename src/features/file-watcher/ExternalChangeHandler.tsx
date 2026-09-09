import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '@/entities/game';
import { useDiskReconcileCoordinator } from './hooks/useFileWatcher';

/**
 * Headless coordinator for Disk Reconcile.
 */
export function ExternalChangeHandler() {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  useDiskReconcileCoordinator(activeGame, queryClient);

  return null;
}
