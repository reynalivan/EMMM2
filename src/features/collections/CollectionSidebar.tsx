import { useActiveGame } from '../../hooks/useActiveGame';
import { useCollectionPreview } from './hooks/useCollections';
import { Layers, Box, Loader2, ShieldAlert } from 'lucide-react';
import type { CollectionPreviewMod } from '../../types/collection';

interface CollectionSidebarProps {
  collectionId: string | null;
  onClose: () => void;
}

export default function CollectionSidebar({ collectionId, onClose }: CollectionSidebarProps) {
  const { activeGame } = useActiveGame();
  const { data: previewMods, isLoading } = useCollectionPreview(
    collectionId,
    activeGame?.id ?? null,
  );

  if (!collectionId) return null;

  // Group mods by object_name
  const groupedMods =
    previewMods?.reduce(
      (acc, mod) => {
        const groupName = mod.object_name || 'Uncategorized';
        if (!acc[groupName]) {
          acc[groupName] = [];
        }
        acc[groupName].push(mod);
        return acc;
      },
      {} as Record<string, CollectionPreviewMod[]>,
    ) ?? {};

  // Sort groups alphabetically
  const sortedGroups = Object.keys(groupedMods).sort();

  return (
    <div className="card bg-base-200/30 border border-white/5 shadow-lg h-[500px] flex flex-col ml-2 md:col-span-4 transition-all duration-300">
      <div className="p-4 border-b border-white/5 flex flex-col bg-base-200/50 rounded-t-2xl">
        <div className="flex justify-between items-center mb-1">
          <h3 className="font-semibold px-1 flex items-center gap-2">
            <Layers size={16} className="text-secondary" />
            Collection Details
          </h3>
          <button
            className="btn btn-xs btn-circle btn-ghost"
            onClick={onClose}
            aria-label="Close details"
          >
            ✕
          </button>
        </div>
        <p className="text-xs text-base-content/50 px-1">Mods that will be applied</p>
      </div>

      <div className="card-body p-0 flex-1 overflow-y-auto custom-scrollbar">
        {isLoading ? (
          <div className="flex justify-center py-12 text-base-content/50">
            <Loader2 size={24} className="animate-spin" />
          </div>
        ) : previewMods?.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-8 text-center h-full">
            <Box size={32} className="text-base-content/20 mb-3" />
            <p className="text-sm text-base-content/50">This collection has no mods.</p>
          </div>
        ) : (
          <div className="p-2 space-y-4">
            {sortedGroups.map((groupName) => (
              <div key={groupName} className="space-y-1">
                <div className="flex items-center gap-2 px-2 pb-1 border-b border-white/5">
                  <span className="text-xs font-semibold text-secondary min-w-[30%] uppercase tracking-wider">
                    {groupName}
                  </span>
                  <span className="badge badge-sm badge-ghost opacity-50 text-[10px]">
                    {groupedMods[groupName].length} mod(s)
                  </span>
                </div>

                <ul className="space-y-px pt-1">
                  {groupedMods[groupName].map((mod) => (
                    <li
                      key={mod.id}
                      className="px-3 py-1.5 text-sm hover:bg-white/5 rounded-md flex justify-between items-center group transition-colors"
                      title={mod.folder_path}
                    >
                      <span className="truncate pr-2 opacity-90 group-hover:opacity-100 flex-1">
                        {mod.actual_name}
                      </span>
                      {!mod.is_safe && (
                        <div title="NSFW">
                          <ShieldAlert size={12} className="text-error opacity-70 shrink-0" />
                        </div>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
