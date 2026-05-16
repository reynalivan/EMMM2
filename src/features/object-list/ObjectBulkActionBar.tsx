import {
  Trash2,
  Pin,
  PinOff,
  Power,
  PowerOff,
  TagIcon,
  Tags,
  X,
  MoreHorizontal,
  Sparkles,
  Star,
  StarOff,
  ShieldCheck,
  ShieldAlert,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../stores/useAppStore';

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
  const safeMode = useAppStore((state) => state.safeMode);

  if (count === 0) return null;

  return (
    <div className="flex items-center justify-between w-full h-8 px-2 bg-primary text-primary-content rounded-md shadow-sm animate-in fade-in zoom-in-95 duration-200">
      {/* Left: Clear + Count */}
      <div className="flex items-center gap-2">
        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={onClear}
          title={t('bulk.clear_selection')}
        >
          <X size={15} />
        </button>
        <span className="text-xs font-semibold tabular-nums">
          {t('bulk.selected_count', { count })}
        </span>
      </div>

      {/* Right: Primary actions + More dropdown */}
      <div className="flex items-center gap-1">
        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={onDelete}
          title={t('bulk.delete_selected')}
          disabled={mutationsDisabled}
        >
          <Trash2 size={15} />
        </button>

        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={() => onPin(true)}
          title={t('bulk.pin_selected')}
          disabled={mutationsDisabled}
        >
          <Pin size={15} />
        </button>

        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={() => onFavorite(true)}
          title={t('bulk.favorite')}
          disabled={mutationsDisabled}
        >
          <Star size={15} />
        </button>

        {/* Adaptive Safety Toggle */}
        {safeMode ? (
          <button
            className="btn btn-xs btn-ghost btn-circle text-warning hover:bg-primary-content/20"
            onClick={() => onMarkSafe(false)}
            title={t('bulk.mark_unsafe')}
            disabled={mutationsDisabled}
          >
            <ShieldAlert size={15} />
          </button>
        ) : (
          <button
            className="btn btn-xs btn-ghost btn-circle text-success hover:bg-primary-content/20"
            onClick={() => onMarkSafe(true)}
            title={t('bulk.mark_safe')}
            disabled={mutationsDisabled}
          >
            <ShieldCheck size={15} />
          </button>
        )}

        {/* Dropdown for secondary actions */}
        <div className="dropdown dropdown-end">
          <div
            tabIndex={0}
            role="button"
            className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
            title={t('bulk.more_actions')}
            aria-disabled={mutationsDisabled}
          >
            <MoreHorizontal size={15} />
          </div>
          <ul
            tabIndex={0}
            className="dropdown-content z-50 menu p-2 shadow bg-base-200 text-base-content rounded-box w-40 mt-1"
          >
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5"
                onClick={() => onPin(false)}
                disabled={mutationsDisabled}
              >
                <PinOff size={14} className="opacity-70" />
                {t('bulk.unpin')}
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-success"
                onClick={onEnable}
                disabled={mutationsDisabled}
              >
                <Power size={14} className="opacity-70" />
                {t('bulk.enable')}
              </button>
            </li>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-warning"
                onClick={onDisable}
                disabled={mutationsDisabled}
              >
                <PowerOff size={14} className="opacity-70" />
                {t('bulk.disable')}
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-info"
                onClick={onAutoRecognize}
                disabled={mutationsDisabled}
              >
                <Sparkles size={14} className="opacity-70" />
                {t('bulk.auto_recognize')}
              </button>
            </li>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5"
                onClick={() => onFavorite(false)}
                disabled={mutationsDisabled}
              >
                <StarOff size={14} className="opacity-70" />
                {t('bulk.unfavorite')}
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5"
                onClick={onAddTags}
                disabled={mutationsDisabled}
              >
                <TagIcon size={14} className="opacity-70" />
                {t('bulk.add_tags')}
              </button>
            </li>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-error"
                onClick={onRemoveTags}
                disabled={mutationsDisabled}
              >
                <Tags size={14} className="opacity-70" />
                {t('bulk.remove_tags')}
              </button>
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
