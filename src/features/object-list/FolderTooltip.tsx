/**
 * FolderTooltip — Hover tooltip for scan review rows.
 * Shows full path, thumbnail preview, and lazy-loaded folder contents.
 */

import { useState, useRef, useCallback, type ReactNode } from 'react';
import * as Tooltip from '@radix-ui/react-tooltip';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Folder, FileText } from 'lucide-react';

interface FolderEntry {
  name: string;
  is_dir: boolean;
}

interface FolderTooltipProps {
  folderPath: string;
  thumbnailPath: string | null;
  gameId: string;
  children: ReactNode;
}

// Module-level cache to avoid re-fetching
const entryCache = new Map<string, FolderEntry[]>();

export default function FolderTooltip({
  folderPath,
  thumbnailPath,
  gameId,
  children,
}: FolderTooltipProps) {
  const [entries, setEntries] = useState<FolderEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const fetchedRef = useRef(false);

  const handleOpen = useCallback(
    (open: boolean) => {
      if (!open || fetchedRef.current) return;
      fetchedRef.current = true;

      // Check cache first
      const cached = entryCache.get(folderPath);
      if (cached) {
        setEntries(cached);
        return;
      }

      setLoading(true);
      invoke<FolderEntry[]>('list_folder_entries_cmd', { folderPath, gameId })
        .then((result) => {
          entryCache.set(folderPath, result);
          setEntries(result);
        })
        .catch(() => setEntries([]))
        .finally(() => setLoading(false));
    },
    [folderPath, gameId],
  );

  return (
    <Tooltip.Provider>
      <Tooltip.Root delayDuration={100} onOpenChange={handleOpen}>
        <Tooltip.Trigger asChild>{children}</Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            align="start"
            sideOffset={6}
            className="z-1000 max-w-xs rounded-lg border border-base-content/10 bg-base-100 shadow-xl p-3 text-xs
              data-[state=delayed-open]:animate-in data-[state=delayed-open]:fade-in-0 data-[state=delayed-open]:zoom-in-95 data-[state=delayed-open]:duration-150
              data-[state=instant-open]:animate-in data-[state=instant-open]:fade-in-0 data-[state=instant-open]:zoom-in-95 data-[state=instant-open]:duration-150
              data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=closed]:duration-100"
          >
            {/* Full path */}
            <p className="font-mono text-[10px] text-base-content/50 break-all mb-2 leading-tight">
              {folderPath}
            </p>

            {/* Thumbnail */}
            {thumbnailPath && (
              <div className="mb-2 rounded overflow-hidden border border-base-300/30">
                <img
                  src={convertFileSrc(thumbnailPath)}
                  alt="Preview"
                  className="w-full h-20 object-cover"
                />
              </div>
            )}

            {/* Folder content */}
            <div className="border-t border-base-content/10 pt-2">
              <p className="text-[10px] uppercase tracking-wider text-base-content/40 font-semibold mb-1">
                Contents
              </p>
              {loading && (
                <div className="flex flex-col gap-1">
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="h-3 bg-base-300/40 rounded animate-pulse w-3/4" />
                  ))}
                </div>
              )}
              {entries && entries.length === 0 && (
                <p className="text-base-content/30 italic">Empty folder</p>
              )}
              {entries && entries.length > 0 && (
                <ul className="flex flex-col gap-0.5 max-h-40 overflow-y-auto">
                  {entries.map((entry) => (
                    <li key={entry.name} className="flex items-center gap-1.5 text-base-content/70">
                      {entry.is_dir ? (
                        <Folder size={11} className="text-warning/70 shrink-0" />
                      ) : (
                        <FileText size={11} className="text-base-content/40 shrink-0" />
                      )}
                      <span className="truncate">{entry.name}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            <Tooltip.Arrow className="fill-base-100" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
