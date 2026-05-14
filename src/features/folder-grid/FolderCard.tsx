import { useState, memo, useCallback, useMemo } from 'react';
import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ContextMenu } from '../../components/ui/ContextMenu';
import type { ModFolder } from '../../types/mod';
import type { WorkspaceExplorerNode } from '../../types/workspace';
import FolderCardContextMenu from './FolderCardContextMenu';
import BulkContextMenu from './BulkContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';
import { useAppStore } from '../../stores/useAppStore';
import { formatWorkspaceWarning } from '../workspace-runtime/workspaceSemantics';
import { buildWorkspaceSwitchPolicy } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { WorkspaceSwitchControl } from '../workspace-runtime/components/WorkspaceSwitchControl';
import { WorkspaceSwitchLabel } from '../workspace-runtime/components/WorkspaceSwitchLabel';
import { maskWorkspaceNodeCapabilities } from '../workspace-runtime/actions/workspaceActionAvailability';
import { getFolderTypeChip } from './FolderTypeChip';
import FolderCardThumbnail from './FolderCardThumbnail';

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
  mutationsDisabled?: boolean;
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
  mutationsDisabled = false,
}: FolderCardProps) {
  const { t } = useTranslation(['grid', 'common']);
  const typeChip = getFolderTypeChip(folder.type_chip, t, 'card');
  const actionFolder = useMemo(
    () => maskWorkspaceNodeCapabilities(folder, mutationsDisabled),
    [folder, mutationsDisabled],
  );
  const primaryWarningText = formatWorkspaceWarning(t, folder.primary_warning);
  const switchPolicy = useMemo(
    () => buildWorkspaceSwitchPolicy(t, actionFolder),
    [actionFolder, t],
  );
  const [imgError, setImgError] = useState(false);
  const [imgLoaded, setImgLoaded] = useState(false);
  const [renameValue, setRenameValue] = useState(folder.name);

  const handleToggleClick = useCallback(
    (e?: React.MouseEvent | React.ChangeEvent) => {
      e?.stopPropagation();
      if (mutationsDisabled) {
        return;
      }

      if (isLockedByParent) {
        onRequestEnableParent?.();
        return;
      }

      onToggleEnabled?.(folder);
    },
    [folder, isLockedByParent, mutationsDisabled, onRequestEnableParent, onToggleEnabled],
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
            onToggle={mutationsDisabled ? undefined : onBulkToggle}
            onDelete={mutationsDisabled ? undefined : onBulkDelete}
            onTag={mutationsDisabled ? undefined : onBulkTag}
            onFavorite={mutationsDisabled ? undefined : onBulkFavorite}
            onSafe={mutationsDisabled ? undefined : onBulkSafe}
            onPin={mutationsDisabled ? undefined : onBulkPin}
            onMoveToObject={mutationsDisabled ? undefined : onBulkMoveToObject}
          />
        ) : (
          <FolderCardContextMenu
            folder={actionFolder}
            onRename={() => !mutationsDisabled && onRename?.(folder)}
            onDelete={() => !mutationsDisabled && onDelete?.(folder)}
            onToggle={() => {
              if (!mutationsDisabled) {
                handleToggleClick();
              }
            }}
            onToggleFavorite={() => !mutationsDisabled && onToggleFavorite?.(folder)}
            onEnableOnlyThis={
              onEnableOnlyThis && !mutationsDisabled ? () => onEnableOnlyThis(folder) : undefined
            }
            onOpenMoveDialog={mutationsDisabled ? undefined : onOpenMoveDialog}
            onNavigate={onNavigate}
            onToggleSafe={mutationsDisabled ? undefined : onToggleSafe}
            onSyncWithDb={
              onSyncWithDb && !mutationsDisabled ? () => onSyncWithDb(folder) : undefined
            }
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
        <FolderCardThumbnail
          folder={folder}
          typeChip={typeChip}
          thumbnailSrc={thumbnailSrc}
          thumbLoading={thumbLoading}
          imgLoaded={imgLoaded}
          imgError={imgError}
          isSelected={isSelected}
          isHiddenByMask={isHiddenByMask}
          isLockedByParent={isLockedByParent}
          isSwitchChecked={switchPolicy.checked}
          hasConflict={hasConflict}
          hasNamingConflict={hasNamingConflict}
          primaryWarningText={primaryWarningText}
          mutationsDisabled={mutationsDisabled}
          onImageError={() => setImgError(true)}
          onImageLoaded={() => setImgLoaded(true)}
          onToggleFavorite={() => {
            if (!mutationsDisabled) {
              onToggleFavorite?.(folder);
            }
          }}
          onToggleSelection={(isShift) => toggleSelection(folder.path, true, isShift)}
        />

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
                node={actionFolder}
                policy={switchPolicy}
                isPending={isSwitchPending || mutationsDisabled}
                size="xs"
                ariaLabel={t('common:actions.toggle')}
                onToggle={() => {
                  handleToggleClick();
                }}
              />
              <div className="flex flex-col">
                <WorkspaceSwitchLabel
                  node={actionFolder}
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
