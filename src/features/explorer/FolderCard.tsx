import { useState, memo } from 'react';
import { Folder, Star, Copy, Package, Layers } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { ContextMenu } from '../../components/ui/ContextMenu';
import type { ModFolder } from '../../types/mod';
import { isNavigable } from '../../types/mod';
import FolderCardContextMenu from './FolderCardContextMenu';
import BulkContextMenu from './BulkContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';

interface FolderCardProps {
  folder: ModFolder;
  isSelected: boolean;
  onNavigate: (name: string) => void;
  toggleSelection: (id: string, multi: boolean) => void;
  clearSelection: () => void;
  onToggleEnabled?: (folder: ModFolder) => void;
  onToggleFavorite?: (folder: ModFolder) => void;
  onRename?: (folder: ModFolder) => void;
  onDelete?: (folder: ModFolder) => void;
  onEnableOnlyThis?: (folder: ModFolder) => void;
  isRenaming?: boolean;
  onRenameSubmit?: (newName: string) => void;
  onRenameCancel?: () => void;
  isFocused?: boolean;
  selectionSize?: number;
  onBulkToggle?: (enable: boolean) => void;
  onBulkDelete?: () => void;
  onBulkTag?: () => void;
  onBulkFavorite?: (favorite: boolean) => void;
  onBulkSafe?: (safe: boolean) => void;
  onBulkPin?: (pin: boolean) => void;
  onBulkMoveToObject?: () => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onToggleSafe?: () => void;
  hasConflict?: boolean;
}

function FolderCardInner({
  folder,
  isSelected,
  onNavigate,
  toggleSelection,
  clearSelection,
  onToggleEnabled,
  onToggleFavorite,
  onRename,
  onDelete,
  onEnableOnlyThis,
  isRenaming = false,
  onRenameSubmit,
  onRenameCancel,
  isFocused = false,
  selectionSize = 0,
  onBulkToggle,
  onBulkDelete,
  onBulkTag,
  onBulkFavorite,
  onBulkSafe,
  onBulkPin,
  onBulkMoveToObject,
  onOpenMoveDialog,
  onToggleSafe,
  hasConflict = false,
}: FolderCardProps) {
  const [imgError, setImgError] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [renameValue, setRenameValue] = useState(folder.name);

  // Lazy thumbnail: resolved per-card via separate backend command
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(folder.path);

  // Convert filesystem path to Tauri asset:// protocol for display
  const thumbnailSrc = thumbnailPath && !imgError ? convertFileSrc(thumbnailPath) : null;

  // Reset image state when thumbnail path changes (e.g. after lazy resolve or update)
  const [prevThumbnailPath, setPrevThumbnailPath] = useState(thumbnailPath);
  if (thumbnailPath !== prevThumbnailPath) {
    setPrevThumbnailPath(thumbnailPath);
    setImgError(false);
    setImgLoaded(false);
  }

  // Sync rename value when folder changes or rename starts
  if (!isRenaming && renameValue !== folder.name) {
    setRenameValue(folder.name);
  }

  const handleRenameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.stopPropagation();
      onRenameSubmit?.(renameValue);
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      onRenameCancel?.();
    }
  };

  const handleClick = (e: React.MouseEvent) => {
    if (!e.ctrlKey && !e.shiftKey) clearSelection();
    toggleSelection(folder.path, e.ctrlKey || e.shiftKey);
  };

  const handleDoubleClick = () => {
    if (folder.is_directory && isNavigable(folder)) {
      onNavigate(folder.folder_name);
    }
  };

  const isBulkSelection = isSelected && selectionSize > 1;

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
          <FolderCardContextMenu
            folder={folder}
            onRename={() => onRename?.(folder)}
            onDelete={() => onDelete?.(folder)}
            onToggle={() => onToggleEnabled?.(folder)}
            onToggleFavorite={() => onToggleFavorite?.(folder)}
            onEnableOnlyThis={onEnableOnlyThis ? () => onEnableOnlyThis(folder) : undefined}
            onOpenMoveDialog={onOpenMoveDialog}
            onNavigate={onNavigate}
            onToggleSafe={onToggleSafe}
          />
        )
      }
    >
      <div
        id={`grid-item-${folder.path}`}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
        className={`
          group relative flex flex-col rounded-lg overflow-hidden cursor-pointer
          transition-all duration-200 border w-full
          ${folder.node_type === 'InternalAssets' ? 'opacity-50' : ''}
          ${
            isSelected
              ? 'border-primary/50 bg-base-200 shadow-md ring-1 ring-primary/50'
              : isFocused
                ? 'border-primary/30 bg-base-200 ring-2 ring-primary'
                : 'border-transparent bg-base-200 hover:bg-base-300 hover:shadow-lg hover:-translate-y-0.5'
          }
        `}
        role="gridcell"
        aria-label={`${folder.name} — ${folder.is_enabled ? 'enabled' : 'disabled'}`}
        tabIndex={0}
      >
        {/* Thumbnail area — 3:4 portrait ratio */}
        <div className="aspect-square bg-base-300/50 relative group overflow-hidden">
          {/* Skeleton Shimmer — visible while thumbnail is loading or image hasn't decoded */}
          {(thumbLoading || (thumbnailSrc && !imgLoaded && !imgError)) && (
            <div className="absolute inset-0 skeleton bg-base-300" />
          )}

          {thumbnailSrc ? (
            <img
              src={thumbnailSrc}
              alt=""
              decoding="async"
              className={`w-full h-full object-cover transition-all duration-500
                ${isSelected ? 'scale-105' : 'scale-100 group-hover:scale-105'}
                ${imgLoaded ? (isSelected ? 'opacity-100' : 'opacity-85 group-hover:opacity-100') : 'opacity-0'}
              `}
              draggable={false}
              onError={() => setImgError(true)}
              onLoad={() => setImgLoaded(true)}
            />
          ) : (
            <Folder
              size={40}
              className={`transition-colors duration-300
                ${isSelected ? 'text-primary' : 'text-base-content/15 group-hover:text-base-content/30'}`}
            />
          )}

          {/* Node type badge overlay */}
          {folder.node_type === 'ModPackRoot' && (
            <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-info/90 text-info-content rounded-md z-10 shadow-sm">
              <Package size={10} />
              <span className="text-[9px] font-bold uppercase">Mod Pack</span>
            </div>
          )}
          {folder.node_type === 'VariantContainer' && (
            <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-secondary/90 text-secondary-content rounded-md z-10 shadow-sm">
              <Layers size={10} />
              <span className="text-[9px] font-bold uppercase">Variants</span>
            </div>
          )}

          {/* Favorite star overlay */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onToggleFavorite?.(folder);
            }}
            className={`absolute top-1.5 right-1.5 p-1 rounded-full transition-all duration-200 z-10
               ${
                 folder.is_favorite
                   ? 'text-warning opacity-100 hover:scale-110'
                   : 'text-base-content/20 opacity-0 group-hover:opacity-100 hover:text-warning hover:scale-110'
               }
             `}
            title={folder.is_favorite ? 'Unfavorite' : 'Favorite'}
          >
            <Star
              size={16}
              className={`drop-shadow-sm ${folder.is_favorite ? 'fill-current' : ''}`}
            />
          </button>

          {/* Misplaced Warning */}
          {folder.is_misplaced && (
            <div
              className="absolute bottom-1.5 right-1.5 p-1 bg-error/90 text-error-content rounded-full z-10 shadow-sm"
              title="Misplaced: Character mismatch detected in info.json"
            >
              <div className="w-2 h-2 rounded-full bg-current animate-ping absolute inset-0 opacity-75"></div>
              <span className="text-[10px] font-bold px-1">!</span>
            </div>
          )}

          {/* Duplicate / Conflict Badge */}
          {hasConflict && (
            <div
              className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/90 text-warning-content rounded-md z-10 shadow-sm"
              title="Hash conflict: shares shader/buffer hashes with another enabled mod"
            >
              <Copy size={10} />
              <span className="text-[9px] font-bold">Conflict</span>
            </div>
          )}
        </div>

        {/* Info area */}
        <div className="p-2.5 flex-1 flex flex-col justify-between min-h-0">
          <h3
            className={`font-medium text-sm truncate leading-tight select-none transition-colors
              ${isSelected ? 'text-primary' : 'text-base-content/80 group-hover:text-base-content'}`}
            title={folder.name}
          >
            {isRenaming ? (
              <input
                autoFocus
                type="text"
                className="input input-xs input-bordered w-full h-6 px-1 text-sm"
                value={renameValue}
                onChange={(e) => setRenameValue(e.target.value)}
                onKeyDown={handleRenameKeyDown}
                onClick={(e) => e.stopPropagation()}
                onBlur={() => onRenameCancel?.()}
              />
            ) : (
              folder.name
            )}
          </h3>

          <div className="flex items-center justify-between mt-1.5">
            <label
              className="flex items-center gap-1.5 cursor-pointer"
              onClick={(e) => e.stopPropagation()}
            >
              <input
                type="checkbox"
                className="toggle toggle-xs toggle-success"
                checked={folder.is_enabled}
                onChange={() => onToggleEnabled?.(folder)}
              />
              <span className="text-[10px] font-semibold text-base-content/40">
                {folder.is_enabled ? 'Enabled' : 'Disabled'}
              </span>
            </label>
            {folder.is_directory && (
              <span className="text-[9px] text-base-content/20 font-bold tracking-wider">DIR</span>
            )}
          </div>
        </div>
      </div>
    </ContextMenu>
  );
}

/** Memoized FolderCard — prevents re-renders when virtualizer recalculates but props are unchanged */
const FolderCard = memo(FolderCardInner);
export default FolderCard;
