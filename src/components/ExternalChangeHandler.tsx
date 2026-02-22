import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../hooks/useActiveGame';
import { folderKeys } from '../hooks/useFolders';
import { thumbnailKeys } from '../hooks/useThumbnail';
import { toast } from '../stores/useToastStore';

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

  // 1. Start Watcher when game changes
  useEffect(() => {
    if (activeGame?.mod_path) {
      invoke('start_watcher_cmd', { path: activeGame.mod_path }).catch((err) =>
        console.error('Failed to start watcher:', err),
      );
    }
  }, [activeGame?.mod_path]);

  // 2. Listen for events
  useEffect(() => {
    const unlistenPromise = listen<ModWatchEvent>('mod_watch:event', (event) => {
      const payload = event.payload;
      console.log('Watcher Event:', payload);

      // Invalidate queries to auto-refresh grid, sidebar objects, and category counts
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });

      if (activeGame?.mod_path) {
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

        if (payload.type === 'Removed' && isModFolder(payload.path)) {
          toast.warning(
            `External Deletion: "${getFolderName(payload.path)}" was removed. Database auto-updated.`,
          );
        } else if (payload.type === 'Created' && isModFolder(payload.path)) {
          toast.info(
            `External Creation: "${getFolderName(payload.path)}" was added. Database auto-updated.`,
          );
        } else if (payload.type === 'Renamed') {
          // If either the old or new name is a mod folder, notify
          if (isModFolder(payload.from) || isModFolder(payload.to)) {
            toast.info(
              `External Rename: "${getFolderName(payload.from)}" renamed to "${getFolderName(payload.to)}". Database auto-updated.`,
            );
          }
        }
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [queryClient, activeGame]);

  // This component handles logical side-effects only, and requires no UI
  return null;
}
