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
  type: 'Created' | 'Modified' | 'Removed' | 'Renamed' | 'Error';
  path?: string;
  from?: string;
  to?: string;
  error?: string;
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

        // Count depth: 0 slashes = root child, 1 slash = Category/Mod
        const depth = (relativeClean.match(/\//g) || []).length;

        // Ignore files (have an extension like .ini, .txt, .dll)
        const lastSegment = relativeClean.split('/').pop() ?? '';
        const hasExtension = lastSegment.includes('.') && lastSegment.lastIndexOf('.') > 0;

        return depth <= 1 && !hasExtension;
      };

      const getFolderName = (p?: string) => p?.split(/[\\/]/).pop() ?? 'Unknown Item';

      // Only queue valid mod folder changes
      const isValidCreated = payload.type === 'Created' && isModFolder(payload.path);
      const isValidRemoved = payload.type === 'Removed' && isModFolder(payload.path);
      const isValidRenamed =
        payload.type === 'Renamed' && (isModFolder(payload.from) || isModFolder(payload.to));

      if (isValidCreated || isValidRemoved || isValidRenamed) {
        eventQueueRef.current.push(payload);
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

        const totals = { created: 0, removed: 0, renamed: 0 };
        queue.forEach((ev: ModWatchEvent) => {
          if (ev.type === 'Created') totals.created++;
          else if (ev.type === 'Removed') totals.removed++;
          else if (ev.type === 'Renamed') totals.renamed++;
        });

        // Sync DB before invalidating queries
        try {
          if (totals.removed > 0 || totals.renamed > 0) {
            await invoke('gc_lost_objects_cmd', { gameId: activeGame.id });
          }
          if (totals.created > 0 || totals.renamed > 0) {
            await invoke('sync_objects_cmd', { gameId: activeGame.id });
          }
        } catch (err) {
          console.error('Watcher sync failed:', err);
        }

        // Invalidate queries ONCE per batch flush
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: thumbnailKeys.all, refetchType: 'none' });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

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
