/**
 * FolderListRow — List view row for a mod folder.
 * Used by FolderGrid in list mode.
 */

import { memo, useState, useEffect } from 'react';
import {
  Folder,
  File,
  ExternalLink,
  Pencil,
  Trash2,
  ToggleLeft,
  Star,
  ArrowRightLeft,
  Copy,
} from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { ModFolder } from '../../types/mod';
import BulkContextMenu from './BulkContextMenu';

import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';

interface FolderListRowProps {
  item: ModFolder;
  isSelected: boolean;
  toggleSelection: (id: string, multi: boolean) => void;
  clearSelection: () => void;
  onToggleEnabled?: (folder: ModFolder) => void;
  onToggleFavorite?: (folder: ModFolder) => void;
  selectionSize?: number;
  onBulkToggle?: (enable: boolean) => void;
  onBulkDelete?: () => void;
  onBulkTag?: () => void;
  onBulkFavorite?: (favorite: boolean) => void;
  onBulkSafe?: (safe: boolean) => void;
  onBulkPin?: (pin: boolean) => void;
  onBulkMoveToObject?: () => void;
  onRename?: (folder: ModFolder) => void;
  onDelete?: (folder: ModFolder) => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  hasConflict?: boolean;
}

function FolderListRowInner({
  item,
  isSelected,
  toggleSelection,
  clearSelection,
  onToggleEnabled,
  onToggleFavorite,
  selectionSize = 0,
  onBulkToggle,
  onBulkDelete,
  onBulkTag,
  onBulkFavorite,
  onBulkSafe,
  onBulkPin,
  onBulkMoveToObject,
  onRename,
  onDelete,
  onOpenMoveDialog,
  hasConflict = false,
}: FolderListRowProps) {
  // Lazy thumbnail: resolved per-row via separate backend command
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(item.path);
  const [imgError, setImgError] = useState(false);
  const thumbnailSrc = thumbnailPath && !imgError ? convertFileSrc(thumbnailPath) : null;
  const isBulkSelection = isSelected && selectionSize > 1;

  // Reset error state when thumbnail path changes
  useEffect(() => {
    setImgError(false);
  }, [thumbnailPath]);

  const handleContextClick = (_: React.MouseEvent) => {
    // If we right click and it's NOT selected, select it (and clear others if no modifier)
    // Actually ContextMenu trigger handles visibility, but we want to ensure selection logic is visually consistent.
    // If user right clicks an unselected item, standard OS behavior is to select it.
    if (!isSelected) {
      clearSelection();
      toggleSelection(item.path, false);
    }
  };

  return (
    <ContextMenu
      content={
        isBulkSelection ? (
          <BulkContextMenu
            count={selectionSize}
            onToggle={onBulkToggle}
            onDelete={onBulkDelete}
            onTag={onBulkTag}
            onFavorite={onBulkFavorite}
            onSafe={onBulkSafe}
            onPin={onBulkPin}
            onMoveToObject={onBulkMoveToObject}
          />
        ) : (
          <>
            <ContextMenuItem
              icon={ExternalLink}
              onClick={async () => {
                const { invoke } = await import('@tauri-apps/api/core');
                invoke('open_in_explorer', { path: item.path }).catch(console.error);
              }}
            >
              Open in Explorer
            </ContextMenuItem>
            <ContextMenuItem icon={Pencil} onClick={() => onRename?.(item)}>
              Rename
            </ContextMenuItem>
            <ContextMenuItem icon={ToggleLeft} onClick={() => onToggleEnabled?.(item)}>
              {item.is_enabled ? 'Disable' : 'Enable'}
            </ContextMenuItem>
            {onOpenMoveDialog && (
              <>
                <ContextMenuSeparator />
                <ContextMenuItem icon={ArrowRightLeft} onClick={() => onOpenMoveDialog(item)}>
                  Move to Object...
                </ContextMenuItem>
              </>
            )}
            <ContextMenuSeparator />
            <ContextMenuItem icon={Trash2} danger onClick={() => onDelete?.(item)}>
              Delete to Trash
            </ContextMenuItem>
          </>
        )
      }
    >
      <div
        id={`grid-item-${item.path}`}
        onClick={(e) => {
          if (!e.ctrlKey && !e.shiftKey) clearSelection();
          toggleSelection(item.path, e.ctrlKey || e.shiftKey);
        }}
        onContextMenu={handleContextClick}
        className={`
        flex items-center gap-3 p-2 rounded-md border cursor-pointer active:scale-[0.99] transition-all h-full
        ${isSelected ? 'border-primary/50 bg-primary/10' : 'border-base-content/5 bg-base-200 hover:bg-base-300'}
      `}
      >
        <div className="w-10 h-10 shrink-0 bg-base-300 rounded-md overflow-hidden flex items-center justify-center select-none border border-base-content/5">
          {thumbLoading ? (
            <div className="w-full h-full skeleton bg-base-300" />
          ) : thumbnailSrc ? (
            <img
              src={thumbnailSrc}
              alt=""
              decoding="async"
              className="w-full h-full object-cover opacity-90"
              draggable={false}
              onError={() => setImgError(true)}
            />
          ) : item.is_directory ? (
            <Folder size={18} className="text-base-content/20" />
          ) : (
            <File size={18} className="text-base-content/20" />
          )}
        </div>

        <div className="min-w-0 flex-1 flex items-center gap-3">
          <div
            className={`text-sm font-medium truncate leading-tight flex-1
            ${isSelected ? 'text-primary' : 'text-base-content/80'}`}
          >
            {item.name}
          </div>

          {/* Conflict badge */}
          {hasConflict && (
            <div
              className="flex items-center gap-0.5 px-1.5 py-0.5 bg-warning/20 text-warning rounded-md shrink-0"
              title="Hash conflict with another enabled mod"
            >
              <Copy size={10} />
              <span className="text-[9px] font-bold">Conflict</span>
            </div>
          )}

          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={(e) => {
                e.stopPropagation();
                onToggleFavorite?.(item);
              }}
              className={`p-1 rounded-full transition-all duration-200
                 ${
                   item.is_favorite
                     ? 'text-warning opacity-100 hover:scale-110'
                     : 'text-base-content/20 opacity-0 group-hover:opacity-100 hover:text-warning hover:scale-110'
                 }
               `}
              title={item.is_favorite ? 'Unfavorite' : 'Favorite'}
            >
              <Star
                size={16}
                className={`drop-shadow-sm ${item.is_favorite ? 'fill-current' : ''}`}
              />
            </button>
            <label
              className="flex items-center gap-1.5 cursor-pointer"
              onClick={(e) => e.stopPropagation()}
            >
              <input
                type="checkbox"
                className="toggle toggle-xs toggle-success"
                checked={item.is_enabled}
                onChange={() => onToggleEnabled?.(item)}
              />
              <span className="text-[10px] font-semibold text-base-content/40 hidden sm:inline">
                {item.is_enabled ? 'Enabled' : 'Disabled'}
              </span>
            </label>
          </div>
        </div>
      </div>
    </ContextMenu>
  );
}

/** Memoized FolderListRow — prevents re-renders when virtualizer recalculates but props are unchanged */
const FolderListRow = memo(FolderListRowInner);
export default FolderListRow;
