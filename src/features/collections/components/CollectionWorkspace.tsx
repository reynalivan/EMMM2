import { useState, useMemo } from 'react';
import {
  Loader2,
  PlayCircle,
  Save,
  ChevronDown,
  ChevronRight,
  Folder,
  ShieldAlert,
  FolderTree,
  Package,
} from 'lucide-react';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useCollectionPreview, useActiveModsPreview } from '../hooks/useCollections';
import { useAppStore } from '../../../stores/useAppStore';
import type { Collection, CollectionPreviewMod } from '../../../types/collection';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';

interface CollectionWorkspaceProps {
  collection: Collection;
  onApply: (collection: Collection) => void;
  isApplying: boolean;
}

// Compact list row for a single mod
export function ModListRow({ mod }: { mod: CollectionPreviewMod }) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);

  useState(() => {
    invoke<string | null>('get_mod_thumbnail', { folderPath: mod.folder_path })
      .then((path) => {
        if (path) setThumbnailUrl(convertFileSrc(path));
      })
      .catch(() => {});
  });

  return (
    <div
      className="flex items-center gap-3 px-3 py-1.5 hover:bg-base-300/30 transition-colors rounded-md group"
      title={mod.folder_path}
    >
      {/* Thumbnail */}
      <div className="w-7 h-7 rounded-md bg-base-300 overflow-hidden shrink-0 flex items-center justify-center border border-base-content/5">
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt="" className="w-full h-full object-cover" />
        ) : (
          <Folder size={12} className="text-base-content/20" />
        )}
      </div>

      {/* Name */}
      <span className="text-xs font-medium truncate flex-1 text-base-content/80 group-hover:text-base-content transition-colors">
        {mod.actual_name}
      </span>

      {/* Badges */}
      <div className="flex items-center gap-1 shrink-0">
        {mod.id.startsWith('nested_') && (
          <span title="Nested mod" className="flex shrink-0">
            <FolderTree size={11} className="text-info/60" />
          </span>
        )}
        {!mod.is_safe && (
          <span title="Unsafe" className="flex shrink-0">
            <ShieldAlert size={11} className="text-error/60" />
          </span>
        )}
      </div>
    </div>
  );
}

export default function CollectionWorkspace({
  collection,
  onApply,
  isApplying,
}: CollectionWorkspaceProps) {
  const { activeGame } = useActiveGame();
  const { safeMode } = useAppStore();
  const isUnsaved = collection.is_last_unsaved;

  // Track which groups are manually COLLAPSED (inverted: empty = all expanded by default)
  const [collapsedObjects, setCollapsedObjects] = useState<Set<string>>(new Set());

  const regularPreviewQuery = useCollectionPreview(
    isUnsaved ? null : collection.id,
    activeGame?.id ?? null,
  );

  const activeModsQuery = useActiveModsPreview(
    isUnsaved ? (activeGame?.id ?? null) : null,
    safeMode,
  );

  const isLoading = isUnsaved ? activeModsQuery.isLoading : regularPreviewQuery.isLoading;
  const previewMods = isUnsaved ? activeModsQuery.data : regularPreviewQuery.data;

  // Group mods by object (memoized to avoid re-computing on every render)
  const groupedObjects = useMemo(() => {
    const mods = previewMods || [];
    const objectsMap = new Map<
      string,
      { id: string; name: string; type: string; mods: CollectionPreviewMod[]; unsafeCount: number }
    >();

    let hasUncategorized = false;
    const uncategorizedMods: CollectionPreviewMod[] = [];
    let uncategorizedUnsafeCount = 0;

    mods.forEach((mod) => {
      if (mod.object_name) {
        const groupKey = mod.object_name; // Use name as the ultimate grouping key

        if (!objectsMap.has(groupKey)) {
          objectsMap.set(groupKey, {
            id: mod.object_id || groupKey, // Keep ID if available for keys
            name: mod.object_name,
            type: mod.object_type || 'Other',
            mods: [],
            unsafeCount: 0,
          });
        }

        const obj = objectsMap.get(groupKey)!;
        // If the group was created by a nested mod (no type), and this one has a type, upgrade it
        if (obj.type === 'Other' && mod.object_type) {
          obj.type = mod.object_type;
        }
        // If the group was created by a nested mod (no id), and this one has an id, upgrade it
        if (obj.id === groupKey && mod.object_id) {
          obj.id = mod.object_id;
        }

        obj.mods.push(mod);
        if (!mod.is_safe) obj.unsafeCount += 1;
      } else {
        hasUncategorized = true;
        uncategorizedMods.push(mod);
        if (!mod.is_safe) uncategorizedUnsafeCount += 1;
      }
    });

    const groups = Array.from(objectsMap.values());
    if (hasUncategorized) {
      groups.push({
        id: 'uncategorized',
        name: 'Uncategorized',
        type: 'Other',
        mods: uncategorizedMods,
        unsafeCount: uncategorizedUnsafeCount,
      });
    }

    const typeOrder = ['Character', 'Weapon', 'UI', 'Other'];
    groups.sort((a, b) => {
      const idxA = typeOrder.indexOf(a.type);
      const idxB = typeOrder.indexOf(b.type);
      if (idxA !== -1 && idxB !== -1 && idxA !== idxB) return idxA - idxB;
      if (idxA !== -1 && idxB === -1) return -1;
      if (idxA === -1 && idxB !== -1) return 1;
      return a.name.localeCompare(b.name);
    });

    return groups;
  }, [previewMods]);

  const mods = previewMods || [];

  if (isLoading) {
    return (
      <div className="flex flex-col h-full bg-base-100 flex-1 relative items-center justify-center min-h-125">
        <Loader2 size={32} className="animate-spin text-primary opacity-50 mb-4" />
        <p className="text-base-content/50">Loading collection details...</p>
      </div>
    );
  }

  const isExpanded = (objectId: string) => !collapsedObjects.has(objectId);

  const toggleExpand = (objectId: string) => {
    setCollapsedObjects((prev) => {
      const next = new Set(prev);
      if (next.has(objectId)) next.delete(objectId);
      else next.add(objectId);
      return next;
    });
  };

  const expandAll = () => setCollapsedObjects(new Set());
  const collapseAll = () => setCollapsedObjects(new Set(groupedObjects.map((o) => o.id)));

  return (
    <div className="flex flex-col h-full w-full relative">
      {/* Workspace Header */}
      <div className="h-14 bg-base-300/50 backdrop-blur-md border-b border-white/5 flex items-center justify-between px-4 shrink-0 z-10">
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <div className="flex flex-col min-w-0">
            <h2 className="font-bold text-sm leading-tight flex items-center gap-2 truncate">
              <span className="truncate">{collection.name}</span>
              {collection.is_last_unsaved && (
                <span className="badge badge-sm badge-warning opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                  Last Unsaved
                </span>
              )}
            </h2>
            <span className="text-[10px] text-base-content/50 truncate">
              {groupedObjects.length} objects • {mods.length} mods
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button onClick={expandAll} className="btn btn-xs btn-ghost text-base-content/50">
            Expand All
          </button>
          <button onClick={collapseAll} className="btn btn-xs btn-ghost text-base-content/50">
            Collapse All
          </button>

          <button
            onClick={() => onApply(collection)}
            disabled={!collection.is_last_unsaved && isApplying}
            className={`btn btn-sm min-w-30 ml-2 ${collection.is_last_unsaved ? 'btn-secondary' : 'btn-primary'}`}
          >
            {collection.is_last_unsaved ? (
              <>
                <Save size={14} />
                Save Collection
              </>
            ) : isApplying ? (
              <Loader2 size={14} className="animate-spin" />
            ) : (
              <>
                <PlayCircle size={14} />
                Apply Collection
              </>
            )}
          </button>
        </div>
      </div>

      {/* Main Workspace Area */}
      <div className="flex-1 overflow-y-auto custom-scrollbar p-4 bg-base-100/50">
        {groupedObjects.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-base-content/40">
            <Package size={48} className="mb-4 opacity-20" />
            <p>Collection is empty.</p>
          </div>
        ) : (
          <div className="max-w-3xl mx-auto space-y-2">
            {groupedObjects.map((obj) => {
              const expanded = isExpanded(obj.id);
              return (
                <div
                  key={obj.id}
                  className="border border-white/5 rounded-lg overflow-hidden bg-base-200/30"
                >
                  {/* Accordion Header */}
                  <button
                    onClick={() => toggleExpand(obj.id)}
                    className="w-full flex items-center justify-between px-3 py-2 hover:bg-base-300/30 transition-colors"
                  >
                    <div className="flex items-center gap-2">
                      <div className="text-base-content/50">
                        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                      </div>
                      <span className="font-semibold text-xs">{obj.name}</span>
                      <span className="text-[9px] text-base-content/40 uppercase tracking-widest">
                        {obj.type}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-[10px] text-base-content/50">
                        {obj.mods.length} mods
                      </span>
                      {obj.unsafeCount > 0 && (
                        <span className="flex items-center gap-0.5 text-[10px] text-error/70">
                          <ShieldAlert size={10} />
                          {obj.unsafeCount}
                        </span>
                      )}
                    </div>
                  </button>

                  {/* Accordion Body (List) */}
                  {expanded && (
                    <div className="border-t border-white/5 py-1">
                      {obj.mods.map((mod) => (
                        <ModListRow key={mod.id} mod={mod} />
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
