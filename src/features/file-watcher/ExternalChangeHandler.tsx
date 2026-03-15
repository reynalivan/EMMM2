import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useWatcherLifecycle, useWatcherEvents, useWatcherReactions } from './hooks';

/**
 * Headless component that acts as the orchestrator for the FileWatcher system.
 *
 * Post-Refactor Architecture:
 * - `useWatcherLifecycle`: Starts/stops the Rust watcher when the active game changes
 * - `useWatcherEvents`: Listens to IPC events, filters them, and batches into a queue
 * - `useWatcherReactions`: Performs React Query invalidations and shows summary toasts
 */
export function ExternalChangeHandler() {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  // 1. Start/Stop watcher on game change
  useWatcherLifecycle(activeGame);

  // 2. Listen, filter, and debounce events into a batch queue
  const batchedEvents = useWatcherEvents(activeGame);

  // 3. React to validated batches (invalidate cache, show toasts)
  useWatcherReactions(batchedEvents, queryClient);

  // This component handles logical side-effects only, and renders nothing
  return null;
}
