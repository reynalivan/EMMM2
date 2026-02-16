import {
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
} from '../../components/ui/ContextMenu';
import {
  Edit,
  ExternalLink,
  Move,
  Pin,
  Power,
  RefreshCw,
  Trash2,
  Star,
  ToggleLeft,
  ToggleRight,
} from 'lucide-react';

/* -------------------------------------------------------------------------------------------------
 * Types
 * -----------------------------------------------------------------------------------------------*/

export type ContextMenuTarget =
  | { type: 'folder'; path: string; name: string; isEnabled: boolean }
  | {
      type: 'object';
      id: string;
      name: string;
      objectType: string;
      enabledCount?: number;
      modCount?: number;
    };

interface ObjectContextMenuProps {
  item: ContextMenuTarget;
  isSyncing: boolean;
  categories: { name: string; label?: string }[];
  onEditObject: (id: string) => void;
  onEditFolder: (path: string) => void;
  onSyncWithDb: (id: string, name: string) => void;
  onDelete: (path: string) => void; // For folders
  onDeleteObject: (id: string) => void; // For objects
  onToggle: (path: string, currentEnabled: boolean) => void;
  onOpen: (path: string) => void;
  onPin: (id: string) => void;
  onFavorite: (path: string) => void;
  onMoveCategory: (id: string, category: string) => void;
  onEnableAll?: (id: string) => void;
  onDisableAll?: (id: string) => void;
}

/* -------------------------------------------------------------------------------------------------
 * Component
 * -----------------------------------------------------------------------------------------------*/

export function ObjectContextMenu({
  item,
  isSyncing,
  categories,
  onEditObject,
  onEditFolder,
  onSyncWithDb,
  onDelete,
  onDeleteObject,
  onToggle,
  onOpen,
  onPin,
  onFavorite,
  onMoveCategory,
  onEnableAll,
  onDisableAll,
}: ObjectContextMenuProps) {
  // 1. Render Object Menu
  if (item.type === 'object') {
    const hasDisabledMods =
      item.modCount !== undefined &&
      item.enabledCount !== undefined &&
      item.enabledCount < item.modCount;
    const hasEnabledMods = (item.enabledCount ?? 0) > 0;

    return (
      <>
        <ContextMenuItem icon={Edit} onClick={() => onEditObject(item.id)}>
          Edit Metadata
        </ContextMenuItem>
        <ContextMenuItem icon={Pin} onClick={() => onPin(item.id)}>
          Pin to Top
        </ContextMenuItem>

        {/* Enable/Disable All Mods */}
        {hasDisabledMods && onEnableAll && (
          <ContextMenuItem icon={ToggleRight} onClick={() => onEnableAll(item.id)}>
            Enable All Mods
          </ContextMenuItem>
        )}
        {hasEnabledMods && onDisableAll && (
          <ContextMenuItem icon={ToggleLeft} onClick={() => onDisableAll(item.id)}>
            Disable All Mods
          </ContextMenuItem>
        )}

        <ContextMenuSub label="Move Category..." icon={Move}>
          {categories.map((cat) => (
            <ContextMenuItem key={cat.name} onClick={() => onMoveCategory(item.id, cat.name)}>
              {cat.label ?? cat.name}
            </ContextMenuItem>
          ))}
        </ContextMenuSub>
        <ContextMenuSeparator />
        <ContextMenuItem
          icon={RefreshCw}
          onClick={() => onSyncWithDb(item.id, item.name)}
          disabled={isSyncing}
        >
          {isSyncing ? 'Syncing...' : 'Sync with DB'}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem icon={Trash2} danger onClick={() => onDeleteObject(item.id)}>
          Delete Object
        </ContextMenuItem>
      </>
    );
  }

  // 2. Render Folder Menu
  return (
    <>
      <ContextMenuItem icon={Power} onClick={() => onToggle(item.path, item.isEnabled)}>
        {item.isEnabled ? 'Disable' : 'Enable'}
      </ContextMenuItem>
      <ContextMenuItem icon={ExternalLink} onClick={() => onOpen(item.path)}>
        Open in Explorer
      </ContextMenuItem>
      <ContextMenuItem icon={Star} onClick={() => onFavorite(item.path)}>
        Favorite
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Edit} onClick={() => onEditFolder(item.path)}>
        Edit Metadata
      </ContextMenuItem>
      <ContextMenuSub label="Move to..." icon={Move}>
        {categories.map((cat) => (
          <ContextMenuItem key={cat.name} onClick={() => onMoveCategory(item.path, cat.name)}>
            {cat.label ?? cat.name}
          </ContextMenuItem>
        ))}
      </ContextMenuSub>
      <ContextMenuItem
        icon={RefreshCw}
        onClick={() => onSyncWithDb(item.path, item.name)}
        disabled={isSyncing}
      >
        {isSyncing ? 'Syncing...' : 'Sync with DB'}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Trash2} danger onClick={() => onDelete(item.path)}>
        Move to Trash
      </ContextMenuItem>
    </>
  );
}
