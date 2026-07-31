import { ArrowRightLeft, Edit, Pin, PinOff, Star, StarOff, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import SharedBulkActionBar from '../../../components/ui/BulkActionBar';

interface BulkActionBarProps {
  count: number;
  onClear: () => void;
  onToggle: (enable: boolean) => void;
  onDelete: () => void;
  onPin: (pin: boolean) => void;
  onFavorite: (favorite: boolean) => void;
  onMarkSafe: (safe: boolean) => void;
  onUpdateInfo: () => void;
  onMoveToObject: () => void;
  mutationsDisabled?: boolean;
}

export default function BulkActionBar({
  count,
  onClear,
  onToggle,
  onDelete,
  onPin,
  onFavorite,
  onMarkSafe,
  onUpdateInfo,
  onMoveToObject,
  mutationsDisabled = false,
}: BulkActionBarProps) {
  const { t } = useTranslation(['grid']);

  return (
    <SharedBulkActionBar
      variant="floating"
      count={count}
      onClear={onClear}
      onMarkSafe={onMarkSafe}
      mutationsDisabled={mutationsDisabled}
      labels={{
        clear: t('bulk.clear_selection'),
        count: t('bulk.selected'),
        safe: t('bulk.safe_title'),
        unsafe: t('bulk.unsafe_title'),
        menuTitle: t('bulk.ops_title'),
      }}
      toggleGroup={{
        tooltip: t('bulk.toggle_status'),
        enableLabel: t('bulk.enable'),
        disableLabel: t('bulk.disable'),
        onToggle,
      }}
      iconActions={[
        { icon: Pin, label: t('bulk.pin_title'), onClick: () => onPin(true) },
        { icon: Star, label: t('bulk.fav_title'), onClick: () => onFavorite(true) },
      ]}
      dropdownActions={[
        { icon: ArrowRightLeft, label: t('bulk.move_object'), onClick: onMoveToObject },
        { icon: Edit, label: t('bulk.edit_metadata'), onClick: onUpdateInfo },
        { icon: PinOff, label: t('bulk.unpin_title'), onClick: () => onPin(false) },
        { icon: StarOff, label: t('bulk.unfav_title'), onClick: () => onFavorite(false) },
        {
          icon: Trash2,
          label: t('bulk.move_trash'),
          onClick: onDelete,
          className: 'text-error hover:bg-error/10',
          dividerBefore: true,
        },
      ]}
    />
  );
}
