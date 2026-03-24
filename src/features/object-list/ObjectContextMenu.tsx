import {
  Edit,
  ExternalLink,
  Trash2,
  FolderOpen,
  Pin,
  PinOff,
  RefreshCcw,
  Power,
  PowerOff,
  FolderTree,
  FolderUp,
  Star,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cn } from '../../lib/utils';
import type { CategoryDef } from '../../types/object';

export interface ContextMenuTarget {
  type: 'object' | 'folder';
  id: string;
  name: string;
  objectType?: string;
  isEnabled: boolean;
  enabledCount: number;
  modCount: number;
  isPinned: boolean;
  category?: string;
}

interface ObjectContextMenuProps {
  item: ContextMenuTarget;
  isSyncing: boolean;
  categories: Pick<CategoryDef, 'name' | 'label'>[];
  onEditObject: (id: string) => void;
  onEditFolder: (id: string) => void;
  onSyncWithDb: (id: string, name: string) => void;
  onDelete: (path: string) => void;
  onDeleteObject: (id: string) => void;
  onToggle: (path: string, currentEnabled: boolean) => void;
  onOpen: (path: string) => void;
  onPin: (id: string) => void;
  onFavorite: (path: string) => void;
  onMoveCategory: (id: string, category: string, type: 'object' | 'folder') => void;
  onRevealInExplorer?: (id: string) => void;
  onEnableObject?: (id: string) => void;
  onDisableObject?: (id: string) => void;
}

export function ObjectContextMenu({
  item,
  isSyncing,
  categories,
  onEditObject,
  onSyncWithDb,
  onDeleteObject,
  onPin,
  onMoveCategory,
  onRevealInExplorer,
  onEnableObject,
  onDisableObject,
  onOpen,
  onFavorite,
  onDelete,
}: ObjectContextMenuProps) {
  const { t } = useTranslation(['objects']);
  const isObject = item.type === 'object';

  return (
    <ul className="menu menu-sm bg-base-200 text-base-content rounded-box w-56 shadow-xl border border-base-300/30 p-1.5 animate-in fade-in zoom-in-95 duration-100">
      {isObject && (
        <>
          <li>
            <button className="flex items-center gap-2 py-2" onClick={() => onEditObject(item.id)}>
              <Edit size={14} className="opacity-70" />
              {t('context.edit_meta')}
            </button>
          </li>
          <li>
            <button
              className="flex items-center gap-2 py-2 text-primary"
              onClick={() => onRevealInExplorer?.(item.id)}
            >
              <ExternalLink size={14} className="opacity-70" />
              {t('context.reveal_explorer')}
            </button>
          </li>
          <li>
            <button className="flex items-center gap-2 py-2" onClick={() => onPin(item.id)}>
              {item.isPinned ? (
                <>
                  <PinOff size={14} className="opacity-70" />
                  {t('context.unpin')}
                </>
              ) : (
                <>
                  <Pin size={14} className="opacity-70" />
                  {t('context.pin_top')}
                </>
              )}
            </button>
          </li>
          <li>
            <button
              className={cn(
                'flex items-center gap-2 py-2',
                item.isEnabled ? 'text-warning' : 'text-success',
              )}
              onClick={() =>
                item.isEnabled ? onDisableObject?.(item.id) : onEnableObject?.(item.id)
              }
            >
              {item.isEnabled ? (
                <>
                  <PowerOff size={14} className="opacity-70" />
                  {t('context.disable')}
                </>
              ) : (
                <>
                  <Power size={14} className="opacity-70" />
                  {t('context.enable')}
                </>
              )}
            </button>
          </li>

          <div className="divider my-1 opacity-50"></div>

          <li className="menu-title px-2 py-1 text-[10px] uppercase tracking-wider opacity-40 font-bold">
            {t('context.move_category')}
          </li>
          <div className="max-h-40 overflow-y-auto custom-scrollbar">
            {categories.map((cat) => (
              <li key={cat.name}>
                <button
                  className={cn(
                    'flex items-center gap-2 py-1.5',
                    item.category === cat.name ? 'bg-primary/10 text-primary font-medium' : '',
                  )}
                  onClick={() => onMoveCategory(item.id, cat.name, 'object')}
                >
                  <FolderTree size={13} className="opacity-50" />
                  {cat.label ?? cat.name}
                </button>
              </li>
            ))}
          </div>

          <div className="divider my-1 opacity-50"></div>

          <li>
            <button
              className={cn('flex items-center gap-2 py-2 opacity-70', isSyncing && 'animate-spin')}
              onClick={() => onSyncWithDb(item.id, item.name)}
              disabled={isSyncing}
            >
              <RefreshCcw size={14} className="opacity-70" />
              {isSyncing ? t('context.syncing') : t('context.sync_db')}
            </button>
          </li>
          <li>
            <button
              className="flex items-center gap-2 py-2 text-error"
              onClick={() => onDeleteObject(item.id)}
            >
              <Trash2 size={14} className="opacity-70" />
              {t('context.delete_object')}
            </button>
          </li>
        </>
      )}

      {!isObject && (
        <>
          <li>
            <button
              className="flex items-center gap-2 py-2 text-primary"
              onClick={() => onOpen(item.id)}
            >
              <FolderOpen size={14} className="opacity-70" />
              {t('context.open_explorer')}
            </button>
          </li>
          <li>
            <button
              className="flex items-center gap-2 py-2"
              onClick={() => onRevealInExplorer?.(item.id)}
            >
              <ExternalLink size={14} className="opacity-70" />
              {t('context.reveal_explorer')}
            </button>
          </li>
          <div className="divider my-1 opacity-50"></div>
          <li>
            <button className="flex items-center gap-2 py-2" onClick={() => onFavorite(item.id)}>
              <Star size={14} className="opacity-70" />
              {t('context.favorite')}
            </button>
          </li>
          <li>
            <button className="flex items-center gap-2 py-2">
              <FolderUp size={14} className="opacity-70" />
              {t('context.move_to')}
            </button>
          </li>
          <li>
            <button
              className="flex items-center gap-2 py-2 text-error"
              onClick={() => onDelete(item.id)}
            >
              <Trash2 size={14} className="opacity-70" />
              {t('context.move_trash')}
            </button>
          </li>
        </>
      )}
    </ul>
  );
}
