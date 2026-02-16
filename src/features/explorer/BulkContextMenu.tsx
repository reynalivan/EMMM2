import { Pencil, Trash2, ToggleLeft } from 'lucide-react';
import { ContextMenuItem, ContextMenuSeparator } from '../../components/ui/ContextMenu';

interface BulkContextMenuProps {
  count: number;
  onToggle?: (enable: boolean) => void;
  onDelete?: () => void;
  onTag?: () => void;
}

export default function BulkContextMenu({
  count,
  onToggle,
  onDelete,
  onTag,
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
      <ContextMenuItem icon={Pencil} onClick={onTag}>
        Add Tags...
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Trash2} danger onClick={onDelete}>
        Delete {count} Items
      </ContextMenuItem>
    </>
  );
}
