import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../hooks/useActiveGame';
import { folderKeys } from '../../hooks/useFolders';
import { thumbnailKeys } from '../../hooks/useThumbnail';
import { toast } from '../../stores/useToastStore';
import { useAppStore } from '../../stores/useAppStore';

interface ModWatchEvent {
  type: 'Created' | 'Modified' | 'Removed' | 'Renamed' | 'StatusChanged' | 'Error';
  path?: string;
  from?: string;
  to?: string;
  from_status?: string;
  to_status?: string;
  error?: string;
  retry_count?: number;
}

export function ExternalChangeHandler() {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const eventQueueRef = useRef<ModWatchEvent[]>([]);

  // 1. Start/Stop Watcher when game changes (Imp 3: lifecycle cleanup)
  useEffect(() => {
    if (activeGame?.mod_path && activeGame?.id) {
      invoke('start_watcher_cmd', { path: activeGame.mod_path, gameId: activeGame.id }).catch(
        (err) => console.error('Failed to start watcher:', err),
      );
    }
    return () => {
      invoke('stop_watcher_cmd').catch((err) => console.error('Failed to stop watcher:', err));
    };
  }, [activeGame?.mod_path, activeGame?.id]);

  // 2. Listen for events
  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    const unlistenPromise = listen<ModWatchEvent>('mod_watch:event', (event) => {
      const payload = event.payload;

      if (!activeGame?.mod_path) return;

      // Skip events during mutation cooldown to prevent race-condition UI glitches
      const cooldownUntil = useAppStore.getState().watcherCooldownUntil;
      if (cooldownUntil && Date.now() < cooldownUntil) {
        return;
      }

      const normalizedRoot = activeGame.mod_path.replace(/\\/g, '/');

          // Helper to determine if a path is a top-level mod folder
      const isModFolder = (targetPath?: string) => {
        if (!targetPath) return false;
        const normalizedEventPath = targetPath.replace(/\\/g, '/');
        if (!normalizedEventPath.startsWith(normalizedRoot)) return false;

        const relative = normalizedEventPath.slice(normalizedRoot.length);
        const relativeClean = relative.startsWith('/') ? relative.slice(1) : relative;

        // Count path components (folders) — matches backend logic in lifecycle.rs:
        // - depth 1 (components.len() == 1) → object folder (e.g., Mods/Alhaitham)
        // - depth 2 (components.len() == 2) → mod folder (e.g., Mods/Alhaitham/Hair)
        const depth = relativeClean ? relativeClean.split('/').length : 0;

  // Ignore files (have an extension like .ini, .txt, .dll)
  const lastSegment = relativeClean.split('/').pop() ?? '';
  const hasExtension = lastSegment.includes('.') && lastSegment.lastIndexOf('.') > 0;

  // Ignore hidden folders (start with .)
  if (lastSegment.startsWith('.')) return false;

  // Accept object folders (depth=1) and mod folders (depth=2)
  // depth <= 1 means: 0 components (invalid) or 1 component (object folder)
  void hasExtension; // kept for reference even if unused in current logic
  return depth === 1 || depth === 2;
};

      const getFolderName = (p?: string) => p?.split(/[\\/]/).pop() ?? 'Unknown Item';

      // Handle error events with retry info
      if (payload.type === 'Error' && payload.retry_count !== undefined) {
        const errorMsg = `Sync failed after ${payload.retry_count} retries: ${payload.error}`;

        // Show toast
        toast.error(errorMsg);

        // Log for debugging (syncErrors queue removed per lint fix)
        console.error('Watcher sync error:', payload);
        return;
      }

      // Only queue valid mod folder changes
      const isValidCreated = payload.type === 'Created' && isModFolder(payload.path);
      const isValidRemoved = payload.type === 'Removed' && isModFolder(payload.path);
      const isValidRenamed =
        payload.type === 'Renamed' && (isModFolder(payload.from) || isModFolder(payload.to));
      const isValidStatusChanged =
        payload.type === 'StatusChanged' && isModFolder(payload.path);

      if (isValidCreated || isValidRemoved || isValidRenamed || isValidStatusChanged) {
        eventQueueRef.current.push(payload);

        // For status changes, show immediate feedback
        if (isValidStatusChanged) {
          const folderName = getFolderName(payload.path);
          const isEnabled = payload.to_status === 'ENABLED';
          const label = isEnabled ? 'enabled' : 'disabled';
          toast.success(`"${folderName}" was ${label} externally.`);
        }
      } else {
        // Non-mod-folder events: debounce invalidation only, no toast
        if (timeoutId === null) {
          timeoutId = setTimeout(() => {
            // Non-mod-folder events (e.g. .ini edits): mark stale only, no force refetch
            queryClient.invalidateQueries({ queryKey: folderKeys.all, refetchType: 'none' });
            queryClient.invalidateQueries({ queryKey: thumbnailKeys.all, refetchType: 'none' });
            timeoutId = null;
          }, 300); // Imp 5: 800→300ms per req-28 AC-28.1.1
        }
        return;
      }

      // Clear existing timeout to extend debounce window
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }

      // Imp 5: reduced from 800ms to 300ms per req-28 AC-28.1.1 (≤500ms latency)
      timeoutId = setTimeout(async () => {
        const queue = eventQueueRef.current;
        if (queue.length === 0) {
          timeoutId = null;
          return;
        }

        const totals = { created: 0, removed: 0, renamed: 0, statusChanged: 0 };
        queue.forEach((ev: ModWatchEvent) => {
          if (ev.type === 'Created') totals.created++;
          else if (ev.type === 'Removed') totals.removed++;
          else if (ev.type === 'Renamed') totals.renamed++;
          else if (ev.type === 'StatusChanged') totals.statusChanged++;
        });

        // Sync DB before invalidating queries
        try {
          // Always GC lost objects if items were removed or renamed
          if (totals.removed > 0 || totals.renamed > 0) {
            await invoke('gc_lost_objects_cmd', { gameId: activeGame.id });
          }

          // Sync objects if items were created, renamed, or status changed
          if (totals.created > 0 || totals.renamed > 0 || totals.statusChanged > 0) {
            await invoke('sync_objects_cmd', { gameId: activeGame.id });
          }
        } catch (err) {
          console.error('Watcher sync failed:', err);
        }

        // Selective query invalidation based on event types
        if (totals.statusChanged > 0) {
          // Status change: only invalidate objects and category counts (no need to refetch folders)
          queryClient.invalidateQueries({ queryKey: ['objects'] });
          queryClient.invalidateQueries({ queryKey: ['category-counts'] });
        } else if (totals.created > 0 || totals.removed > 0 || totals.renamed > 0) {
          // Structure change: invalidate all queries
          queryClient.invalidateQueries({ queryKey: folderKeys.all });
          queryClient.invalidateQueries({ queryKey: ['objects'] });
          queryClient.invalidateQueries({ queryKey: thumbnailKeys.all, refetchType: 'none' });
          queryClient.invalidateQueries({ queryKey: ['category-counts'] });
        }

        // Imp 2: only show toast if user is actively viewing mods (req-05 line 76)
        const workspaceView = useAppStore.getState().workspaceView;
        if (workspaceView !== 'mods') {
          eventQueueRef.current = [];
          timeoutId = null;
          return;
        }

        if (queue.length === 1) {
          // Exactly 1 valid event -> Show specific toast
          const ev = queue[0];
          if (ev.type === 'Created') {
            toast.info(`"${getFolderName(ev.path)}" was added externally. View refreshed.`);
          } else if (ev.type === 'Removed') {
            toast.warning(`"${getFolderName(ev.path)}" was removed externally. View refreshed.`);
          } else if (ev.type === 'Renamed') {
            toast.info(
              `"${getFolderName(ev.from)}" renamed to "${getFolderName(ev.to)}" externally. View refreshed.`,
            );
          }
        } else {
          // Multiple events -> Show generic summary toast
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

        // Reset queue
        eventQueueRef.current = [];
        timeoutId = null;
      }, 300); // Imp 5: 800→300ms per req-28 AC-28.1.1
    });

    return () => {
      if (timeoutId !== null) clearTimeout(timeoutId);
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [queryClient, activeGame]);

  // This component handles logical side-effects only, and requires no UI
  return null;
}
