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
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation(['grid']);

  return (
    <>
      <div className="px-2 py-1 text-xs font-semibold opacity-50 select-none">
        {t('context.selected_count', { count })}
      </div>
      <ContextMenuSeparator />
      <ContextMenuItem icon={ToggleLeft} onClick={() => onToggle?.(true)}>
        {t('context.enable_selected')}
      </ContextMenuItem>
      <ContextMenuItem icon={ToggleLeft} onClick={() => onToggle?.(false)}>
        {t('context.disable_selected')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Star} onClick={() => onFavorite?.(true)}>
        {t('context.favorite_selected')}
      </ContextMenuItem>
      <ContextMenuItem icon={Star} onClick={() => onFavorite?.(false)}>
        {t('context.unfavorite_selected')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={ShieldCheck} onClick={() => onSafe?.(true)}>
        {t('context.mark_safe')}
      </ContextMenuItem>
      <ContextMenuItem icon={ShieldOff} onClick={() => onSafe?.(false)}>
        {t('context.mark_unsafe')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pin} onClick={() => onPin?.(true)}>
        {t('context.pin_selected')}
      </ContextMenuItem>
      <ContextMenuItem icon={Pin} onClick={() => onPin?.(false)}>
        {t('context.unpin_selected')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pencil} onClick={onTag}>
        {t('context.add_tags')}
      </ContextMenuItem>
      {onMoveToObject && (
        <ContextMenuItem icon={ArrowRightLeft} onClick={onMoveToObject}>
          {t('context.move_to_object')}
        </ContextMenuItem>
      )}
      <ContextMenuSeparator />
      <ContextMenuItem icon={Trash2} danger onClick={onDelete}>
        {t('context.delete_items', { count })}
      </ContextMenuItem>
    </>
  );
}
