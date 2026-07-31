import {
  Trash2,
  Pin,
  PinOff,
  Power,
  PowerOff,
  TagIcon,
  Tags,
  Sparkles,
  Star,
  StarOff,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import SharedBulkActionBar from '../../../components/ui/BulkActionBar';

interface ObjectBulkActionBarProps {
  count: number;
  onDelete: () => void;
  onPin: (pin: boolean) => void;
  onEnable: () => void;
  onDisable: () => void;
  onAddTags: () => void;
  onRemoveTags: () => void;
  onAutoRecognize: () => void;
  onFavorite: (fav: boolean) => void;
  onMarkSafe: (safe: boolean) => void;
  onClear: () => void;
  mutationsDisabled?: boolean;
}

export default function ObjectBulkActionBar({
  count,
  onDelete,
  onPin,
  onEnable,
  onDisable,
  onAddTags,
  onRemoveTags,
  onAutoRecognize,
  onFavorite,
  onMarkSafe,
  onClear,
  mutationsDisabled = false,
}: ObjectBulkActionBarProps) {
  const { t } = useTranslation(['objects']);

  return (
    <SharedBulkActionBar
      variant="inline"
      count={count}
      onClear={onClear}
      onMarkSafe={onMarkSafe}
      mutationsDisabled={mutationsDisabled}
      labels={{
        clear: t('bulk.clear_selection'),
        count: t('bulk.selected_count', { count }),
        safe: t('bulk.mark_safe'),
        unsafe: t('bulk.mark_unsafe'),
        more: t('bulk.more_actions'),
      }}
      iconActions={[
        { icon: Trash2, label: t('bulk.delete_selected'), onClick: onDelete },
        { icon: Pin, label: t('bulk.pin_selected'), onClick: () => onPin(true) },
        { icon: Star, label: t('bulk.favorite'), onClick: () => onFavorite(true) },
      ]}
      dropdownActions={[
        { icon: PinOff, label: t('bulk.unpin'), onClick: () => onPin(false) },
        {
          icon: Power,
          label: t('bulk.enable'),
          onClick: onEnable,
          className: 'text-success',
          dividerBefore: true,
        },
        { icon: PowerOff, label: t('bulk.disable'), onClick: onDisable, className: 'text-warning' },
        {
          icon: Sparkles,
          label: t('bulk.auto_recognize'),
          onClick: onAutoRecognize,
          className: 'text-info',
          dividerBefore: true,
        },
        { icon: StarOff, label: t('bulk.unfavorite'), onClick: () => onFavorite(false) },
        { icon: TagIcon, label: t('bulk.add_tags'), onClick: onAddTags, dividerBefore: true },
        {
          icon: Tags,
          label: t('bulk.remove_tags'),
          onClick: onRemoveTags,
          className: 'text-error',
        },
      ]}
    />
  );
}
