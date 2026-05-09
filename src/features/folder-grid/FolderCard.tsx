import { useState, memo, useCallback, useMemo } from 'react';
import {
  Folder,
  Star,
  Copy,
  Package,
  Layers,
  AlertTriangle,
  PowerOff,
  Lock,
  type LucideIcon,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ContextMenu } from '../../components/ui/ContextMenu';
import type { ModFolder } from '../../types/mod';
import type { WorkspaceExplorerNode, WorkspaceTypeChip } from '../../types/workspace';
import FolderCardContextMenu from './FolderCardContextMenu';
import BulkContextMenu from './BulkContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';
import { useAppStore } from '../../stores/useAppStore';
import { formatWorkspaceWarning } from '../workspace-runtime/workspaceSemantics';
import { buildWorkspaceSwitchPolicy } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { WorkspaceSwitchControl } from '../workspace-runtime/components/WorkspaceSwitchControl';
import { WorkspaceSwitchLabel } from '../workspace-runtime/components/WorkspaceSwitchLabel';

interface FolderCardProps {
  folder: WorkspaceExplorerNode;
  isSelected: boolean;
  isActive?: boolean;
  onNavigate: (name: string) => void;
  toggleSelection: (id: string, multi: boolean, isShift?: boolean) => void;
  onActivate?: (path: string) => void;
  onToggleEnabled?: (folder: WorkspaceExplorerNode) => void;
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
  isSwitchPending?: boolean;
}

function getTypeChip(
  typeChip: WorkspaceTypeChip | null,
  t: ReturnType<typeof useTranslation>['t'],
): { label: string; className: string; icon: LucideIcon } | null {
  if (typeChip === 'mod_pack') {
    return {
      label: t('card.mod_pack'),
      className: 'bg-info/90 text-info-content',
      icon: Package,
    };
  }

  if (typeChip === 'variant') {
    return {
      label: t('card.variants'),
      className: 'bg-secondary/90 text-secondary-content',
      icon: Layers,
    };
  }

  if (typeChip === 'flat_mod') {
    return {
      label: t('card.flat_mod'),
      className: 'bg-base-300/90 text-base-content/80',
      icon: Folder,
    };
  }

  return null;
}

function FolderCardInner({
  folder,
  isSelected,
  isActive,
  onNavigate,
  toggleSelection,
  onActivate,
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
  isSwitchPending = false,
}: FolderCardProps) {
  const { t } = useTranslation(['grid', 'common']);
  const typeChip = getTypeChip(folder.type_chip, t);
  const primaryWarningText = formatWorkspaceWarning(t, folder.primary_warning);
  const switchPolicy = useMemo(() => buildWorkspaceSwitchPolicy(t, folder), [folder, t]);
  const [imgError, setImgError] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [renameValue, setRenameValue] = useState(folder.name);

  const handleToggleClick = useCallback(
    (e?: React.MouseEvent | React.ChangeEvent) => {
      e?.stopPropagation();
      if (isLockedByParent) {
        onRequestEnableParent?.();
        return;
      }

      onToggleEnabled?.(folder);
    },
    [folder, isLockedByParent, onRequestEnableParent, onToggleEnabled],
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
    if (folder.display_mode === 'internal_assets') {
      return;
    }

    if (e.ctrlKey || e.shiftKey) {
      toggleSelection(folder.path, true, e.shiftKey);
    } else {
      onActivate?.(folder.path);
    }
  };

  const handleDoubleClick = () => {
    if (folder.is_directory && folder.can_navigate) {
      onNavigate(folder.folder_name);
    }
  };

  const isBulkSelection =
    isSelected && selectionSize > 1 && useAppStore.getState().activePane === 'folderGrid';
  const hasNamingConflict = !!folder.conflict_state;

  // Leak guard only: main workspace grid should already be corridor-filtered by the backend.
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
          ${!folder.is_effectively_active || isLockedByParent ? 'opacity-[0.75] grayscale-[0.8]' : ''}
          ${
            isActive
              ? 'border-primary/60 bg-primary/10 shadow-md ring-1 ring-primary/60'
              : hasNamingConflict
                ? 'border-warning/60 ring-1 ring-warning/40'
                : isSelected
                  ? 'border-primary/50 bg-base-200 shadow-md ring-1 ring-primary/50'
                  : isFocused
                    ? 'border-primary/30 bg-base-200 ring-2 ring-primary'
                    : 'border-transparent bg-base-200 hover:bg-base-300 hover:shadow-lg hover:-translate-y-0.5'
          }
        `}
        role="gridcell"
        aria-label={t(
          switchPolicy.checked ? 'card.aria_label_enabled' : 'card.aria_label_disabled',
          {
            name: folder.name,
          },
        )}
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
          {!isLockedByParent && typeChip && (
            <div
              className={`absolute top-1.5 left-1.5 flex items-center gap-1 rounded-md px-1.5 py-0.5 shadow-sm z-10 ${typeChip.className}`}
            >
              <typeChip.icon size={10} />
              <span className="text-[9px] font-bold uppercase">{typeChip.label}</span>
            </div>
          )}

          {/* Disabled Power Off Overlay */}
          {!switchPolicy.checked && (
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
              title={
                primaryWarningText || folder.warnings.join('\n') || t('card.corrupt_ini_title')
              }
            >
              <AlertTriangle size={10} />
              <span className="text-[9px] font-bold uppercase">{t('badges.corrupt')}</span>
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
              ${isActive || isSelected ? 'text-primary' : 'text-base-content/80 group-hover:text-base-content'}
              ${!switchPolicy.checked ? 'line-through text-base-content/70' : ''}
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
              <WorkspaceSwitchControl
                node={folder}
                policy={switchPolicy}
                isPending={isSwitchPending}
                size="xs"
                ariaLabel={t('common:actions.toggle')}
                onToggle={() => {
                  handleToggleClick();
                }}
              />
              <div className="flex flex-col">
                <WorkspaceSwitchLabel
                  node={folder}
                  policy={switchPolicy}
                  className="text-[10px] font-semibold text-base-content/60 leading-none"
                />
                {isLockedByParent && switchPolicy.checked && (
                  <span className="text-[8px] text-warning font-bold animate-pulse mt-0.5 italic flex items-center gap-0.5">
                    <AlertTriangle size={8} />
                    {t('card.inherited_lock_warning')}
                  </span>
                )}
              </div>
            </label>
            {folder.can_navigate && (
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
