import {
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
} from '../../components/ui/ContextMenu';
import {
  Edit,
  ExternalLink,
  FolderOpen,
  Move,
  Pin,
  PinOff,
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
      isEnabled: boolean;
      enabledCount?: number;
      modCount?: number;
      isPinned?: boolean;
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
  onMoveCategory: (id: string, category: string, type: 'object' | 'folder') => void;
  onRevealInExplorer?: (id: string) => void;
  onEnableObject?: (id: string) => void;
  onDisableObject?: (id: string) => void;
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
  onRevealInExplorer,
  onEnableObject,
  onDisableObject,
}: ObjectContextMenuProps) {
  // 1. Render Object Menu
  if (item.type === 'object') {
    return (
      <>
        <ContextMenuItem icon={Edit} onClick={() => onEditObject(item.id)}>
          Edit Metadata
        </ContextMenuItem>
        {onRevealInExplorer && (
          <ContextMenuItem icon={FolderOpen} onClick={() => onRevealInExplorer(item.id)}>
            Reveal in File Explorer
          </ContextMenuItem>
        )}
        <ContextMenuItem icon={item.isPinned ? PinOff : Pin} onClick={() => onPin(item.id)}>
          {item.isPinned ? 'Unpin Objects' : 'Pin to Top'}
        </ContextMenuItem>

        {/* Enable/Disable Object — show only the relevant action */}
        {!item.isEnabled && onEnableObject && (
          <ContextMenuItem icon={ToggleRight} onClick={() => onEnableObject(item.id)}>
            Enable
          </ContextMenuItem>
        )}
        {item.isEnabled && onDisableObject && (
          <ContextMenuItem icon={ToggleLeft} onClick={() => onDisableObject(item.id)}>
            Disable
          </ContextMenuItem>
        )}

        <ContextMenuSub label="Move Category..." icon={Move}>
          {categories.map((cat) => (
            <ContextMenuItem
              key={cat.name}
              onClick={() => onMoveCategory(item.id, cat.name, 'object')}
            >
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
          <ContextMenuItem
            key={cat.name}
            onClick={() => onMoveCategory(item.path, cat.name, 'folder')}
          >
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
