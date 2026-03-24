import { useState, memo, useCallback } from 'react';
import { Folder, Star, Copy, Package, Layers, AlertTriangle, PowerOff, Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ContextMenu } from '../../components/ui/ContextMenu';
import type { ModFolder } from '../../types/mod';
import { isNavigable } from '../../types/mod';
import FolderCardContextMenu from './FolderCardContextMenu';
import BulkContextMenu from './BulkContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';
import { useDebounceCallback } from '../../hooks/useDebounceCallback';
import { useAppStore } from '../../stores/useAppStore';

interface FolderCardProps {
  folder: ModFolder;
  isSelected: boolean;
  onNavigate: (name: string) => void;
  toggleSelection: (id: string, multi: boolean, isShift?: boolean) => void;
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
  onSyncWithDb?: (folder: ModFolder) => void;
  hasConflict?: boolean;
  /** True when an ancestor folder in the current path has DISABLED prefix */
  isLockedByParent?: boolean;
  /** Called when user tries to toggle while locked — opens Enable Parent dialog */
  onRequestEnableParent?: () => void;
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
  onSyncWithDb,
  hasConflict = false,
  isLockedByParent = false,
  onRequestEnableParent,
}: FolderCardProps) {
  const { t } = useTranslation(['grid']);
  const [imgError, setImgError] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [renameValue, setRenameValue] = useState(folder.name);

  // Optimistic local state for toggle to allow rapid clicking visually without backend spam
  const [localEnabled, setLocalEnabled] = useState(folder.is_enabled);
  const [prevFolderEnabled, setPrevFolderEnabled] = useState(folder.is_enabled);

  // Sync prop to state without triggering an effect cascade (standard React pattern)
  if (folder.is_enabled !== prevFolderEnabled) {
    setPrevFolderEnabled(folder.is_enabled);
    setLocalEnabled(folder.is_enabled);
  }

  const commitToggle = useDebounceCallback((f: ModFolder, nextState: boolean) => {
    // Only invoke backend RPC if state drifted from actual
    if (f.is_enabled !== nextState) {
      // Because useToggleMod toggles implicitly, we must ensure we are toggling in the right direction
      onToggleEnabled?.(f);
    }
  }, 400);

  const handleToggleClick = useCallback(
    (e?: React.MouseEvent | React.ChangeEvent) => {
      e?.stopPropagation();
      // If locked by ancestor, open Enable Parent dialog instead of toggling
      if (isLockedByParent) {
        onRequestEnableParent?.();
        return;
      }
      const nextState = !localEnabled;
      setLocalEnabled(nextState);
      commitToggle(folder, nextState);
    },
    [localEnabled, folder, commitToggle, isLockedByParent, onRequestEnableParent],
  );

  // Lazy thumbnail: resolved per-card via separate backend command
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(
    activeGameId || '',
    folder.path,
  );

  const thumbnailSrc = thumbnailPath && !imgError ? thumbnailPath : null;

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
      const finalVal = renameValue.trim();
      if (finalVal) onRenameSubmit?.(finalVal);
      else onRenameCancel?.();
    } else if (e.key === 'Escape') {
      e.stopPropagation();
      onRenameCancel?.();
    }
  };

  const handleClick = (e: React.MouseEvent) => {
    if (folder.node_type === 'InternalAssets') {
      return;
    }

    if (!e.ctrlKey && !e.shiftKey) clearSelection();
    toggleSelection(folder.path, e.ctrlKey || e.shiftKey, e.shiftKey);
  };

  const handleDoubleClick = () => {
    if (folder.is_directory && isNavigable(folder)) {
      onNavigate(folder.folder_name);
    }
  };

  const isBulkSelection =
    isSelected && selectionSize > 1 && useAppStore.getState().activePane === 'folderGrid';
  const hasNamingConflict = !!folder.conflict_state;

  // Visual masking: Hide NSFW mods when Safe Mode is active
  const { safeMode } = useAppStore();
  const isHiddenByMask = safeMode && !folder.is_safe;

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
            onToggle={handleToggleClick}
            onToggleFavorite={() => onToggleFavorite?.(folder)}
            onEnableOnlyThis={onEnableOnlyThis ? () => onEnableOnlyThis(folder) : undefined}
            onOpenMoveDialog={onOpenMoveDialog}
            onNavigate={onNavigate}
            onToggleSafe={onToggleSafe}
            onSyncWithDb={onSyncWithDb ? () => onSyncWithDb(folder) : undefined}
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
          ${!localEnabled || isLockedByParent ? 'opacity-[0.75] grayscale-[0.8]' : ''}
          ${
            hasNamingConflict
              ? 'border-warning/60 ring-1 ring-warning/40'
              : isSelected
                ? 'border-primary/50 bg-base-200 shadow-md ring-1 ring-primary/50'
                : isFocused
                  ? 'border-primary/30 bg-base-200 ring-2 ring-primary'
                  : 'border-transparent bg-base-200 hover:bg-base-300 hover:shadow-lg hover:-translate-y-0.5'
          }
        `}
        role="gridcell"
        aria-label={t(localEnabled ? 'card.aria_label_enabled' : 'card.aria_label_disabled', {
          name: folder.name,
        })}
        tabIndex={0}
      >
        {/* Thumbnail area — 3:4 portrait ratio */}
        <div className="aspect-square bg-base-300/50 relative group overflow-hidden flex items-center justify-center">
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
                ${isHiddenByMask ? 'blur-xl' : ''}
              `}
              draggable={false}
              onError={() => setImgError(true)}
              onLoad={() => setImgLoaded(true)}
            />
          ) : (
            <Folder
              size={40}
              className={`transition-colors duration-300
                ${isSelected ? 'text-primary' : 'text-base-content/15 group-hover:text-base-content/30'}
                ${isHiddenByMask ? 'blur-lg' : ''}`}
            />
          )}

          {/* LOCKED badge — shown when an ancestor folder is disabled */}
          {isLockedByParent && (
            <div
              className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/85 text-warning-content rounded-md z-10 shadow-sm"
              title={t('card.locked_by_parent')}
            >
              <Lock size={10} />
              <span className="text-[9px] font-bold">{t('card.locked_badge')}</span>
            </div>
          )}

          {/* Node type badge overlay — only when not locked (avoid badge overlap) */}
          {!isLockedByParent && folder.node_type === 'ModPackRoot' && (
            <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-info/90 text-info-content rounded-md z-10 shadow-sm">
              <Package size={10} />
              <span className="text-[9px] font-bold uppercase">{t('card.mod_pack')}</span>
            </div>
          )}
          {!isLockedByParent && folder.node_type === 'VariantContainer' && (
            <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-secondary/90 text-secondary-content rounded-md z-10 shadow-sm">
              <Layers size={10} />
              <span className="text-[9px] font-bold uppercase">{t('card.variants')}</span>
            </div>
          )}

          {/* Disabled Power Off Overlay */}
          {!localEnabled && (
            <div className="absolute inset-0 flex items-center justify-center bg-overlay-mask z-10 pointer-events-none">
              <PowerOff size={24} className="text-base-content/90 drop-shadow-sm" />
            </div>
          )}

          {/* Favorite star overlay — shifted left slightly to make room for checkbox in top-right */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onToggleFavorite?.(folder);
            }}
            className={`absolute top-1.5 right-8 p-1 rounded-full transition-all duration-200 z-10
               ${
                 folder.is_favorite
                   ? 'text-warning opacity-100 hover:scale-110'
                   : 'text-base-content/20 opacity-0 group-hover:opacity-100 hover:text-warning hover:scale-110'
               }
             `}
            title={t(folder.is_favorite ? 'card.unfavorite' : 'card.favorite')}
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
              title={t('card.misplaced_title')}
            >
              <div className="w-2 h-2 rounded-full bg-current animate-ping absolute inset-0 opacity-75"></div>
              <span className="text-[10px] font-bold px-1">!</span>
            </div>
          )}

          {/* Duplicate / Conflict Badge */}
          {hasConflict && (
            <div
              className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/90 text-warning-content rounded-md z-10 shadow-sm"
              title={t('card.hash_conflict_title')}
            >
              <Copy size={10} />
              <span className="text-[9px] font-bold">{t('card.conflict')}</span>
            </div>
          )}

          {/* Naming Conflict Badge (DISABLED X / X both present) */}
          {hasNamingConflict && !hasConflict && (
            <div
              className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/90 text-warning-content rounded-md z-10 shadow-sm animate-pulse"
              title={t('card.name_conflict_title')}
            >
              <AlertTriangle size={10} />
              <span className="text-[9px] font-bold">{t('card.name_conflict')}</span>
            </div>
          )}

          {/* Corrupt INI Warning Badge */}
          {!hasNamingConflict && !hasConflict && folder.warnings.length > 0 && (
            <div
              className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-error/90 text-error-content rounded-md z-10 shadow-sm"
              title={folder.warnings.join('\n') || t('card.corrupt_ini_title')}
            >
              <AlertTriangle size={10} />
              <span className="text-[9px] font-bold uppercase">CORRUPT</span>
            </div>
          )}

          {/* Bulk Multi-Select Checkbox Overlay: positioned in top-right corner */}
          <div
            className={`absolute top-1.5 right-1.5 transition-all duration-200 z-20
              ${isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-90 group-hover:opacity-100 group-hover:scale-100'}`}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-primary border-2 shadow-sm bg-base-100"
              checked={isSelected}
              onChange={(e) => {
                e.stopPropagation();
                const isShift =
                  e.nativeEvent instanceof MouseEvent && (e.nativeEvent as MouseEvent).shiftKey;
                toggleSelection(folder.path, true, isShift);
              }}
              onClick={(e) => e.stopPropagation()}
            />
          </div>
        </div>

        {/* Info area */}
        <div className="p-2.5 flex-1 flex flex-col justify-between min-h-0">
          <h3
            className={`font-medium text-sm truncate leading-tight select-none transition-colors
              ${isSelected ? 'text-primary' : 'text-base-content/80 group-hover:text-base-content'}
              ${!localEnabled ? 'line-through text-base-content/70' : ''}
              ${isHiddenByMask ? 'blur-xs text-base-content/40' : ''}`}
            title={isHiddenByMask ? t('card.hidden_mod_title') : folder.name}
          >
            {isRenaming ? (
              <input
                autoFocus
                type="text"
                className="input input-xs input-bordered w-full h-6 px-1 text-sm"
                value={renameValue}
                onChange={(e) => {
                  let val = e.target.value;
                  if (/^(disabled|disable|dis)[_\-\s]+/i.test(val)) {
                    val = val.replace(/^(disabled|disable|dis)[_\-\s]+/i, '');
                  }
                  setRenameValue(val);
                }}
                onKeyDown={handleRenameKeyDown}
                onClick={(e) => e.stopPropagation()}
                onBlur={() => onRenameCancel?.()}
              />
            ) : isHiddenByMask ? (
              t('card.hidden_mod')
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
                className={`toggle toggle-xs ${isLockedByParent ? 'toggle-warning opacity-80' : 'toggle-success'}`}
                checked={localEnabled}
                onChange={handleToggleClick}
              />
              <div className="flex flex-col">
                <span className="text-[10px] font-semibold text-base-content/60 leading-none">
                  {isLockedByParent
                    ? t('card.locked_by_parent')
                    : t(localEnabled ? 'card.status_enabled' : 'card.status_disabled')}
                </span>
                {isLockedByParent && localEnabled && (
                  <span className="text-[8px] text-warning font-bold animate-pulse mt-0.5 italic flex items-center gap-0.5">
                    <AlertTriangle size={8} />
                    {t('card.inherited_lock_warning')}
                  </span>
                )}
              </div>
            </label>
            {isNavigable(folder) && (
              <span className="text-[9px] text-base-content/20 font-bold tracking-wider">
                {t('card.dir_label')}
              </span>
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
