import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../hooks/useActiveGame';
import { useAppStore } from '../stores/useAppStore';
import { folderKeys } from '../hooks/useFolders';
import { thumbnailKeys } from '../hooks/useThumbnail';
import { AlertTriangle } from 'lucide-react';

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

  const [removedPath, setRemovedPath] = useState<string | null>(null);

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

      // Invalidate queries to refresh grid, sidebar objects, and category counts.
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: thumbnailKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });

      if (payload.type === 'Removed' && payload.path && activeGame?.mod_path) {
        // Normalize paths for comparison (simple replace)
        const normalizedEventPath = payload.path.replace(/\\/g, '/');
        const normalizedRoot = activeGame.mod_path.replace(/\\/g, '/');

        // Check if it is a direct child
        // e.g. root: "C:/Mods"
        // event: "C:/Mods/ModA" -> Direct child (Yes)
        // event: "C:/Mods/ModA/file.ini" -> Nested (No)

        if (normalizedEventPath.startsWith(normalizedRoot)) {
          const relative = normalizedEventPath.slice(normalizedRoot.length);
          // relative starts with / ?
          const relativeClean = relative.startsWith('/') ? relative.slice(1) : relative;

          // If relative path has no more slashes, it's a direct child (folder or file in root)
          if (!relativeClean.includes('/')) {
            // It's a root item.
            // Could be a file in root (which we ignore usually) or a folder.
            // Since it's removed, we can't check isDirectory.
            // But "Remove from database" implies it WAS a mod.
            // We show the modal.
            setRemovedPath(payload.path);
          }
        }
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [queryClient, activeGame]); // Added activeGame dependency

  const handleConfirmRemove = async () => {
    if (!removedPath || !activeGame) return;

    try {
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      useAppStore.getState().setSelectedObjectType(null);
    } catch (e) {
      console.error('Failed to handle external deletion:', e);
    }
    setRemovedPath(null);
  };

  const handleLocatePath = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        title: 'Locate New Path for Mod',
      });
      if (selected) {
        // User picked a new location — refresh queries so the grid picks up the mod
        queryClient.invalidateQueries({ queryKey: folderKeys.all });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      }
    } catch (e) {
      console.error('Failed to open directory picker:', e);
    }
    setRemovedPath(null);
  };

  const dialogRef = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (removedPath && !dialog.open) dialog.showModal();
    else if (!removedPath && dialog.open) dialog.close();
  }, [removedPath]);

  if (!removedPath) return null;

  const folderName = removedPath.split(/[\\/]/).pop() ?? removedPath;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      onClose={() => setRemovedPath(null)}
    >
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-sm">
        <div className="flex items-start gap-3">
          <div className="p-2 rounded-lg bg-warning/10 text-warning">
            <AlertTriangle size={20} />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-base text-base-content">
              External Deletion Detected
            </h3>
            <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
              &quot;{folderName}&quot; was removed or renamed externally. What would you like to do?
            </p>
          </div>
        </div>

        <div className="modal-action mt-4 flex-wrap gap-2">
          <button className="btn btn-sm btn-ghost" onClick={() => setRemovedPath(null)}>
            Dismiss
          </button>
          <button className="btn btn-sm btn-outline" onClick={handleLocatePath}>
            Locate New Path
          </button>
          <button className="btn btn-sm btn-warning" onClick={handleConfirmRemove}>
            Remove from DB
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={() => setRemovedPath(null)}>close</button>
      </form>
    </dialog>
  );
}
