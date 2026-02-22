import {
  Pencil,
  Trash2,
  ToggleLeft,
  Star,
  ShieldCheck,
  ShieldOff,
  Pin,
  ArrowRightLeft,
} from 'lucide-react';
import { ContextMenuItem, ContextMenuSeparator } from '../../components/ui/ContextMenu';

interface BulkContextMenuProps {
  count: number;
  onToggle?: (enable: boolean) => void;
  onDelete?: () => void;
  onTag?: () => void;
  onFavorite?: (favorite: boolean) => void;
  onSafe?: (safe: boolean) => void;
  onPin?: (pin: boolean) => void;
  onMoveToObject?: () => void;
}

export default function BulkContextMenu({
  count,
  onToggle,
  onDelete,
  onTag,
  onFavorite,
  onSafe,
  onPin,
  onMoveToObject,
}: BulkContextMenuProps) {
  return (
    <>
      <div className="px-2 py-1 text-xs font-semibold opacity-50 select-none">
        {count} items selected
      </div>
      <ContextMenuSeparator />
      <ContextMenuItem icon={ToggleLeft} onClick={() => onToggle?.(true)}>
        Enable Selected
      </ContextMenuItem>
      <ContextMenuItem icon={ToggleLeft} onClick={() => onToggle?.(false)}>
        Disable Selected
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Star} onClick={() => onFavorite?.(true)}>
        Favorite Selected
      </ContextMenuItem>
      <ContextMenuItem icon={Star} onClick={() => onFavorite?.(false)}>
        Unfavorite Selected
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={ShieldCheck} onClick={() => onSafe?.(true)}>
        Mark Safe
      </ContextMenuItem>
      <ContextMenuItem icon={ShieldOff} onClick={() => onSafe?.(false)}>
        Mark Unsafe
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pin} onClick={() => onPin?.(true)}>
        Pin Selected
      </ContextMenuItem>
      <ContextMenuItem icon={Pin} onClick={() => onPin?.(false)}>
        Unpin Selected
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pencil} onClick={onTag}>
        Add Tags...
      </ContextMenuItem>
      {onMoveToObject && (
        <ContextMenuItem icon={ArrowRightLeft} onClick={onMoveToObject}>
          Move to Object...
        </ContextMenuItem>
      )}
      <ContextMenuSeparator />
      <ContextMenuItem icon={Trash2} danger onClick={onDelete}>
        Delete {count} Items
      </ContextMenuItem>
    </>
  );
}
