import {
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
} from '../../components/ui/ContextMenu';
import {
  Edit,
  Trash2,
  FolderOpen,
  Pin,
  PinOff,
  RefreshCw,
  Move,
  ToggleLeft,
  ToggleRight,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { CategoryDef } from '../../types/object';
import type { WorkspaceCapabilities } from '../../types/workspace';
import type { WorkspaceObjectActionPolicy } from '../workspace-runtime/actions/workspaceActionPolicy';

export interface ContextMenuTarget {
  type: 'object';
  id: string;
  name: string;
  objectType: string;
  isEnabled: boolean;
  enabledCount: number;
  modCount: number;
  isPinned: boolean;
  category?: string;
  capabilities: WorkspaceCapabilities;
  actionPolicy?: WorkspaceObjectActionPolicy;
}

interface ObjectContextMenuProps {
  item: ContextMenuTarget;
  isSyncing: boolean;
  categories: Pick<CategoryDef, 'name' | 'label'>[];
  onEditObject: (id: string) => void;
  onSyncWithDb: (id: string, name: string) => void;
  onDeleteObject: (id: string) => void;
  onPin: (id: string) => void;
  onMoveCategory: (id: string, category: string, type: 'object') => void;
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
}: ObjectContextMenuProps) {
  const { t } = useTranslation(['objects']);
  const actionPolicy = item.actionPolicy ?? {
    canEdit: item.capabilities.can_edit_metadata || item.capabilities.can_rename,
    canReveal: item.capabilities.can_reveal_in_explorer,
    canPin: item.capabilities.can_pin,
    canMoveCategory: item.capabilities.can_move_category,
    canSync: item.capabilities.can_sync,
    canDelete: item.capabilities.can_delete,
    canEnable: item.capabilities.can_toggle && !item.isEnabled,
    canDisable: item.capabilities.can_toggle && item.isEnabled,
  };

  return (
    <>
      {actionPolicy.canEdit ? (
        <ContextMenuItem icon={Edit} onClick={() => onEditObject(item.id)}>
          {t('context.edit_meta')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canReveal && onRevealInExplorer ? (
        <ContextMenuItem icon={FolderOpen} onClick={() => onRevealInExplorer(item.id)}>
          {t('context.reveal_explorer')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canPin ? (
        <ContextMenuItem icon={item.isPinned ? PinOff : Pin} onClick={() => onPin(item.id)}>
          {item.isPinned ? t('context.unpin') : t('context.pin_top')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canDisable && onDisableObject ? (
        <ContextMenuItem icon={ToggleLeft} onClick={() => onDisableObject(item.id)}>
          {t('context.disable')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canEnable && onEnableObject ? (
        <ContextMenuItem icon={ToggleRight} onClick={() => onEnableObject(item.id)}>
          {t('context.enable')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canMoveCategory ? (
        <ContextMenuSub label={t('context.move_category')} icon={Move}>
          {categories.map((cat) => (
            <ContextMenuItem
              key={cat.name}
              onClick={() => onMoveCategory(item.id, cat.name, 'object')}
            >
              {cat.label ?? cat.name}
            </ContextMenuItem>
          ))}
        </ContextMenuSub>
      ) : null}
      <ContextMenuSeparator />
      {actionPolicy.canSync ? (
        <ContextMenuItem
          icon={RefreshCw}
          onClick={() => onSyncWithDb(item.id, item.name)}
          disabled={isSyncing}
        >
          {isSyncing ? t('context.syncing') : t('context.sync_db')}
        </ContextMenuItem>
      ) : null}
      {actionPolicy.canDelete ? (
        <>
          <ContextMenuSeparator />
          <ContextMenuItem icon={Trash2} danger onClick={() => onDeleteObject(item.id)}>
            {t('context.delete_object')}
          </ContextMenuItem>
        </>
      ) : null}
    </>
  );
}
