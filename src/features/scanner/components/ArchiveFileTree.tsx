import { useState, useMemo } from 'react';
import { ChevronDown, ChevronRight, Folder, File } from 'lucide-react';

interface Entry {
  path: string;
  isDir: boolean;
  size: number;
}

interface Props {
  entries: Entry[];
  totalCount: number;
}

/** Collapsible file tree preview for archive contents. */
export default function ArchiveFileTree({ entries, totalCount }: Props) {
  const [expanded, setExpanded] = useState(false);

  // Group entries by top-level folder
  const grouped = useMemo(() => {
    const map = new Map<string, { count: number; totalSize: number }>();
    let rootFiles = 0;

    for (const entry of entries) {
      const normalized = entry.path.replace(/\\/g, '/');
      const parts = normalized.split('/').filter(Boolean);

      if (parts.length <= 1 && !entry.isDir) {
        rootFiles++;
        continue;
      }

      const topFolder = parts[0];
      const existing = map.get(topFolder) || { count: 0, totalSize: 0 };
      existing.count++;
      existing.totalSize += entry.size;
      map.set(topFolder, existing);
    }

    return { folders: map, rootFiles };
  }, [entries]);

  if (entries.length === 0) return null;

  const displayLimit = 100;
  const sortedFolders = [...grouped.folders.entries()].sort((a, b) => a[0].localeCompare(b[0]));

  return (
    <div className="mt-1">
      <button
        type="button"
        className="btn btn-xs btn-ghost text-base-content/50 gap-1"
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
        <span className="text-[10px]">
          Contents ({totalCount} file{totalCount !== 1 ? 's' : ''})
        </span>
      </button>

      {expanded && (
        <div className="ml-2 mt-1 border-l border-base-300 pl-2 space-y-0.5 max-h-40 overflow-y-auto">
          {sortedFolders.slice(0, displayLimit).map(([folder, info]) => (
            <div key={folder} className="flex items-center gap-1.5 text-[11px]">
              <Folder className="w-3 h-3 text-primary/60 shrink-0" />
              <span className="truncate font-mono">{folder}/</span>
              <span className="text-base-content/40 shrink-0">
                ({info.count} file{info.count !== 1 ? 's' : ''})
              </span>
            </div>
          ))}

          {grouped.rootFiles > 0 && (
            <div className="flex items-center gap-1.5 text-[11px]">
              <File className="w-3 h-3 text-base-content/40 shrink-0" />
              <span className="text-base-content/50 italic">
                {grouped.rootFiles} loose file{grouped.rootFiles !== 1 ? 's' : ''}
              </span>
            </div>
          )}

          {sortedFolders.length > displayLimit && (
            <div className="text-[10px] text-base-content/40 italic">
              …and {sortedFolders.length - displayLimit} more folders
            </div>
          )}

          {totalCount > entries.length && (
            <div className="text-[10px] text-base-content/40 italic">
              Showing {entries.length} of {totalCount} entries
            </div>
          )}
        </div>
      )}
    </div>
  );
}
