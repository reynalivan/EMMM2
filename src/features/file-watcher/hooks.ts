import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { QueryClient } from '@tanstack/react-query';
import { folderKeys } from '../../hooks/useFolders';
import { thumbnailKeys } from '../../hooks/useThumbnail';
import { relativePathFromRoot } from '../../lib/pathKey';
import { toast } from '../../stores/useToastStore';
import { useAppStore } from '../../stores/useAppStore';
import { reconcileActiveCollection } from '../collections/utils/reconcileActiveCollection';
import type { GameConfig } from '../../types/game';

/**
 * Typed IPC payload from the Rust backend.
 * Matches `WatchEventPayload` in `src-tauri/src/services/scanner/watcher/mod.rs`
 */
export type WatchEventPayload =
  | { type: 'Created'; path: string }
  | { type: 'Modified'; path: string }
  | { type: 'Removed'; path: string }
  | { type: 'Renamed'; from: string; to: string }
  | { type: 'StatusChanged'; path: string; from_status: string; to_status: string }
  | { type: 'Error'; error: string; path?: string; retry_count?: number };

/**
 * Hook 1: Manages the lifecycle (start/stop) of the Rust filesystem watcher
 * when the active game changes.
 */
export function useWatcherLifecycle(activeGame: GameConfig | null) {
  useEffect(() => {
    let cancelled = false;
    const init = async () => {
      // Always stop existing watcher first to prevent duplicates
      await invoke('stop_watcher_cmd').catch(() => {});
      if (cancelled) return;

      if (activeGame?.mod_path && activeGame?.id) {
        await invoke('start_watcher_cmd', {
          path: activeGame.mod_path,
          gameId: activeGame.id,
        }).catch((err) => console.error('Failed to start watcher:', err));
      }
    };
    init();

    return () => {
      cancelled = true;
      invoke('stop_watcher_cmd').catch(() => {});
    };
  }, [activeGame?.mod_path, activeGame?.id]);
}

/**
 * Internal helper to check if a path is a mod folder (depth 1 or 2)
 * vs a file/unrelated folder.
 */
export function isModFolder(targetPath: string | undefined, rootPath: string | undefined): boolean {
  if (!targetPath || !rootPath) return false;

  const relativeClean = relativePathFromRoot(rootPath, targetPath);
  if (relativeClean === null) return false;

  // Depth 1 = object folder (Mods/Alhaitham)
  // Depth 2 = mod folder (Mods/Alhaitham/Hair)
  const depth = relativeClean ? relativeClean.split('/').length : 0;

  // Ignore files (have an extension)
  const lastSegment = relativeClean.split('/').pop() ?? '';
  const hasExtension = lastSegment.includes('.') && lastSegment.lastIndexOf('.') > 0;

  // Ignore hidden folders
  if (lastSegment.startsWith('.')) return false;
  if (hasExtension) return false;

  return depth === 1 || depth === 2;
}

/**
 * Hook 2: Listens for typed IPC events from the backend, filters invalid ones,
 * and batches them into a queue using a 300ms debounce window.
 */
export function useWatcherEvents(activeGame: GameConfig | null): WatchEventPayload[] {
  const [batchedEvents, setBatchedEvents] = useState<WatchEventPayload[]>([]);
  const eventQueueRef = useRef<WatchEventPayload[]>([]);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!activeGame?.mod_path) return;

    const unlistenPromise = listen<WatchEventPayload[]>('mod_watch:events_batch', (event) => {
      const payloads = event.payload;

      // Skip events during mutation cooldown to prevent UI glitches
      const cooldownUntil = useAppStore.getState().watcherCooldownUntil;
      if (cooldownUntil && Date.now() < cooldownUntil) {
        return;
      }

      let hasValidEvents = false;

      // Process the batch
      for (const payload of payloads) {
        // Handle raw backend errors immediately
        if (payload.type === 'Error') {
          if (payload.retry_count !== undefined) {
            toast.error(`Sync failed after ${payload.retry_count} retries: ${payload.error}`);
          } else {
            toast.error(`Watcher error: ${payload.error}`);
          }
          console.error('Watcher runtime error:', payload);
          continue;
        }

        const isValid = (() => {
          switch (payload.type) {
            case 'Created':
            case 'Removed':
            case 'StatusChanged':
              return isModFolder(payload.path, activeGame.mod_path);
            case 'Renamed':
              return (
                isModFolder(payload.from, activeGame.mod_path) ||
                isModFolder(payload.to, activeGame.mod_path)
              );
            default:
              return false;
          }
        })();

        if (isValid) {
          eventQueueRef.current.push(payload);
          hasValidEvents = true;

          // Immediate feedback for status changes
          if (payload.type === 'StatusChanged') {
            const folderName = payload.path.split(/[\\/]/).pop() ?? 'Unknown Item';
            const isEnabled = payload.to_status === 'ENABLED';
            toast.success(`"${folderName}" was ${isEnabled ? 'enabled' : 'disabled'} externally.`);
          }
        } else if (payload.type === 'Modified') {
          // Files like .ini getting modified don't trigger structure changes,
          // but we want to enqueue them to trigger cache invalidations
          eventQueueRef.current.push(payload);
          hasValidEvents = true;
        }
      }

      if (!hasValidEvents) return; // Ignore if entire batch was invalid

      // Debounce logic (300ms window)
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current);
      }

      timeoutRef.current = setTimeout(() => {
        const queue = [...eventQueueRef.current];
        eventQueueRef.current = [];
        timeoutRef.current = null;

        if (queue.length > 0) {
          setBatchedEvents(queue);
        }
      }, 300);
    });

    return () => {
      if (timeoutRef.current !== null) clearTimeout(timeoutRef.current);
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.mod_path]);

  // Clear events once they're observed to prevent loops, but doing that in rendering
  // is bad practice. We expose the raw batch state. Downstream reacts to changes.
  return batchedEvents;
}

/**
 * Hook 3: Reacts to a batch of validated watcher events by invalidating React Query
 * caches and showing summary toasts.
 */
export function useWatcherReactions(events: WatchEventPayload[], queryClient: QueryClient) {
  useEffect(() => {
    if (events.length === 0) return;

    const totals = { created: 0, removed: 0, renamed: 0, statusChanged: 0, modified: 0 };
    events.forEach((ev) => {
      if (ev.type === 'Created') totals.created++;
      else if (ev.type === 'Removed') totals.removed++;
      else if (ev.type === 'Renamed') totals.renamed++;
      else if (ev.type === 'StatusChanged') totals.statusChanged++;
      else if (ev.type === 'Modified') totals.modified++;
    });

    const hasStructureChange = totals.created > 0 || totals.removed > 0 || totals.renamed > 0;

    // React Query Invalidation
    if (totals.statusChanged > 0 && !hasStructureChange) {
      // Status change only: invalidate objects and category counts
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      void reconcileActiveCollection();
    }

    if (hasStructureChange) {
      // Structure change: invalidate all queries
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      void reconcileActiveCollection();
    }

    if (totals.modified > 0 && !hasStructureChange && totals.statusChanged === 0) {
      // Only modified files (e.g. .ini, thumbnail edits): silent invalidation
      queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.all, refetchType: 'none' });
    }

    // Only show toast if user is actively viewing mods (req-05)
    // and there was a structural change
    const workspaceView = useAppStore.getState().workspaceView;
    if (workspaceView !== 'mods' || (!hasStructureChange && totals.statusChanged === 0)) {
      return;
    }

    const getFolderName = (p?: string) => p?.split(/[\\/]/).pop() ?? 'Unknown Item';

    // Filter out "Modified" events for the toast counting
    const structuralEvents = events.filter(
      (e) => e.type !== 'Modified' && e.type !== 'StatusChanged',
    );

    if (structuralEvents.length === 1) {
      const ev = structuralEvents[0];
      if (ev.type === 'Created') {
        toast.info(`"${getFolderName(ev.path)}" was added externally. View refreshed.`);
      } else if (ev.type === 'Removed') {
        toast.warning(`"${getFolderName(ev.path)}" was removed externally. View refreshed.`);
      } else if (ev.type === 'Renamed') {
        toast.info(
          `"${getFolderName(ev.from)}" renamed to "${getFolderName(ev.to)}" externally. View refreshed.`,
        );
      }
    } else if (structuralEvents.length > 1) {
      const summaryParts = [];
      if (totals.created > 0) summaryParts.push(`${totals.created} added`);
      if (totals.removed > 0) summaryParts.push(`${totals.removed} removed`);
      if (totals.renamed > 0) summaryParts.push(`${totals.renamed} renamed`);

      const msg = `External changes detected: ${summaryParts.join(', ')}. View refreshed.`;
      if (totals.removed > 0) {
        toast.warning(msg);
      } else {
        toast.info(msg);
      }
    }
  }, [events, queryClient]); // Trigger whenever batch changes
}
