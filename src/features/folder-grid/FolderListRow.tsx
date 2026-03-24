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
  Package,
  Layers,
  AlertTriangle,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../../types/mod';
import BulkContextMenu from './BulkContextMenu';
import { useAppStore } from '../../stores/useAppStore';
import { commands } from '../../lib/bindings';

import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../components/ui/ContextMenu';
import { useThumbnail } from '../../hooks/useThumbnail';

interface FolderListRowProps {
  item: ModFolder;
  isSelected: boolean;
  toggleSelection: (id: string, multi: boolean, isShift?: boolean) => void;
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
  onToggleSafe?: () => void;
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
  onToggleSafe,
  hasConflict = false,
}: FolderListRowProps) {
  const { t } = useTranslation(['grid']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { data: thumbnailPath, isLoading: thumbLoading } = useThumbnail(
    activeGameId || '',
    item.path,
  );
  const [imgError, setImgError] = useState(false);
  const thumbnailSrc = thumbnailPath && !imgError ? thumbnailPath : null;
  const isBulkSelection =
    isSelected && selectionSize > 1 && useAppStore.getState().activePane === 'folderGrid';

  // Reset error state when thumbnail path changes
  useEffect(() => {
    setImgError(false);
  }, [thumbnailPath]);

  const handleContextClick = (_: React.MouseEvent) => {
    if (item.node_type === 'InternalAssets') {
      return;
    }

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
                if (activeGameId) {
                  commands
                    .openInExplorer({ gameId: activeGameId, path: item.path })
                    .catch(console.error);
                }
              }}
            >
              {t('context.open_explorer')}
            </ContextMenuItem>
            <ContextMenuItem icon={Pencil} onClick={() => onRename?.(item)}>
              {t('context.rename')}
            </ContextMenuItem>
            <ContextMenuItem icon={ToggleLeft} onClick={() => onToggleEnabled?.(item)}>
              {item.is_enabled ? t('context.disable') : t('context.enable')}
            </ContextMenuItem>
            {onOpenMoveDialog && (
              <>
                <ContextMenuSeparator />
                <ContextMenuItem icon={ArrowRightLeft} onClick={() => onOpenMoveDialog(item)}>
                  {t('context.move_to_object')}
                </ContextMenuItem>
              </>
            )}
            <ContextMenuSeparator />
            <ContextMenuItem icon={Trash2} danger onClick={() => onDelete?.(item)}>
              {t('context.delete_trash')}
            </ContextMenuItem>
            {onToggleSafe && (
              <>
                <ContextMenuSeparator />
                <ContextMenuItem icon={item.is_safe ? ToggleLeft : Star} onClick={onToggleSafe}>
                  {item.is_safe ? t('context.mark_unsafe') : t('context.mark_safe')}
                </ContextMenuItem>
              </>
            )}
          </>
        )
      }
    >
      <div
        id={`grid-item-${item.path}`}
        onClick={(e) => {
          if (item.node_type === 'InternalAssets') {
            return;
          }

          if (!e.ctrlKey && !e.shiftKey) clearSelection();
          toggleSelection(item.path, e.ctrlKey || e.shiftKey, e.shiftKey);
        }}
        onContextMenu={handleContextClick}
        className={`
        flex items-center gap-3 p-2 rounded-md border cursor-pointer active:scale-[0.99] transition-all h-full
        ${item.node_type === 'InternalAssets' ? 'opacity-50' : ''}
        ${!item.is_enabled ? 'opacity-[0.65] grayscale-[0.8]' : ''}
        ${isSelected ? 'border-primary/50 bg-primary/10' : 'border-base-content/5 bg-base-200 hover:bg-base-300'}
      `}
      >
        <div className="group relative w-10 h-10 shrink-0 bg-base-300 rounded-md overflow-hidden flex items-center justify-center select-none border border-base-content/5">
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

          {/* Bulk Multi-Select Checkbox Overlay: positioned in top-right corner */}
          <div
            className={`absolute top-0.5 right-0.5 transition-all duration-200 z-20
              ${isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-75 group-hover:opacity-100 group-hover:scale-100'}`}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-primary checkbox-xs border shadow-sm bg-base-100"
              checked={isSelected}
              onChange={(e) => {
                e.stopPropagation();
                const isShift =
                  e.nativeEvent instanceof MouseEvent && (e.nativeEvent as MouseEvent).shiftKey;
                toggleSelection(item.path, true, isShift);
              }}
              onClick={(e) => e.stopPropagation()}
            />
          </div>
        </div>

        <div className="min-w-0 flex-1 flex items-center gap-3">
          <div
            className={`text-sm font-medium truncate leading-tight flex-1
            ${isSelected ? 'text-primary' : 'text-base-content/80'}
            ${!item.is_enabled ? 'line-through text-base-content/50' : ''}`}
          >
            {item.name}
          </div>

          {/* Node type badge */}
          {item.node_type === 'ModPackRoot' && (
            <div className="flex items-center gap-0.5 px-1.5 py-0.5 bg-info/20 text-info rounded-md shrink-0">
              <Package size={10} />
              <span className="text-[9px] font-bold">{t('card.mod_pack')}</span>
            </div>
          )}
          {item.node_type === 'VariantContainer' && (
            <div className="flex items-center gap-0.5 px-1.5 py-0.5 bg-secondary/20 text-secondary rounded-md shrink-0">
              <Layers size={10} />
              <span className="text-[9px] font-bold">{t('card.variants')}</span>
            </div>
          )}

          {/* Conflict badge */}
          {hasConflict && (
            <div
              className="flex items-center gap-0.5 px-1.5 py-0.5 bg-warning/20 text-warning rounded-md shrink-0"
              title={t('card.hash_conflict_title')}
            >
              <Copy size={10} />
              <span className="text-[9px] font-bold">{t('card.conflict')}</span>
            </div>
          )}

          {/* Corrupt badge */}
          {!hasConflict && item.warnings.length > 0 && (
            <div
              className="flex items-center gap-0.5 px-1.5 py-0.5 bg-error/20 text-error rounded-md shrink-0"
              title={item.warnings.join('\n') || t('card.corrupt_ini_title')}
            >
              <AlertTriangle size={10} />
              <span className="text-[9px] font-bold">CORRUPT</span>
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
              title={t(item.is_favorite ? 'card.unfavorite' : 'card.favorite')}
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
                {t(item.is_enabled ? 'card.status_enabled' : 'card.status_disabled')}
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
