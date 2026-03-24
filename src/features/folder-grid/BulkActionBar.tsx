import {
  Trash2,
  Pin,
  PinOff,
  Power,
  PowerOff,
  MoreHorizontal,
  X,
  Star,
  StarOff,
  Edit,
  ShieldCheck,
  ShieldAlert,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../stores/useAppStore';

interface BulkActionBarProps {
  count: number;
  onClear: () => void;
  onToggle: (enable: boolean) => void;
  onDelete: () => void;
  onPin: (pin: boolean) => void;
  onFavorite: (favorite: boolean) => void;
  onMarkSafe: (safe: boolean) => void;
  onUpdateInfo: () => void;
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
}: BulkActionBarProps) {
  const { t } = useTranslation(['grid']);
  const safeMode = useAppStore((state) => state.safeMode);

  if (count === 0) return null;

  return (
    <div className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center justify-between min-w-[320px] h-12 px-4 bg-primary text-primary-content rounded-full shadow-2xl animate-in slide-in-from-bottom-4 fade-in duration-300">
      {/* Left: Clear + Count */}
      <div className="flex items-center gap-3">
        <button
          className="btn btn-sm btn-ghost btn-circle text-primary-content hover:bg-primary-content/20 transition-colors"
          onClick={onClear}
          title={t('bulk.clear_selection')}
        >
          <X size={18} />
        </button>
        <div className="flex flex-col leading-none">
          <span className="text-sm font-bold tabular-nums">{count}</span>
          <span className="text-[10px] uppercase tracking-wider opacity-70 font-semibold">
            {t('bulk.selected')}
          </span>
        </div>
      </div>

      <div className="h-6 w-px bg-primary-content/20 mx-4" />

      {/* Center: Main Actions */}
      <div className="flex items-center gap-2">
        <div className="tooltip tooltip-top" data-tip={t('bulk.toggle_status')}>
          <div className="join bg-primary-content/10 rounded-full p-0.5">
            <button
              className="btn btn-xs join-item btn-ghost text-success hover:bg-success hover:text-success-content border-none h-7 px-3"
              onClick={() => onToggle(true)}
            >
              <Power size={14} className="mr-1" />
              {t('bulk.enable')}
            </button>
            <div className="w-px h-4 bg-primary-content/10 self-center" />
            <button
              className="btn btn-xs join-item btn-ghost text-warning hover:bg-warning hover:text-warning-content border-none h-7 px-3"
              onClick={() => onToggle(false)}
            >
              <PowerOff size={14} className="mr-1" />
              {t('bulk.disable')}
            </button>
          </div>
        </div>

        <button
          className="btn btn-sm btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={() => onPin(true)}
          title={t('bulk.pin_title')}
        >
          <Pin size={18} />
        </button>

        <button
          className="btn btn-sm btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={() => onFavorite(true)}
          title={t('bulk.fav_title')}
        >
          <Star size={18} />
        </button>

        {/* Adaptive Safety Toggle */}
        {safeMode ? (
          <button
            className="btn btn-sm btn-ghost btn-circle text-warning hover:bg-warning hover:text-warning-content border-none"
            onClick={() => onMarkSafe(false)}
            title={t('bulk.unsafe_title')}
          >
            <ShieldAlert size={18} />
          </button>
        ) : (
          <button
            className="btn btn-sm btn-ghost btn-circle text-success hover:bg-success hover:text-success-content border-none"
            onClick={() => onMarkSafe(true)}
            title={t('bulk.safe_title')}
          >
            <ShieldCheck size={18} />
          </button>
        )}
      </div>

      <div className="h-6 w-px bg-primary-content/20 mx-4" />

      {/* Right: More Actions */}
      <div className="flex items-center gap-1">
        <div className="dropdown dropdown-top dropdown-end">
          <div
            tabIndex={0}
            role="button"
            className="btn btn-sm btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          >
            <MoreHorizontal size={18} />
          </div>
          <ul
            tabIndex={0}
            className="dropdown-content z-60 menu p-2 shadow-xl bg-base-200 text-base-content rounded-box w-52 mb-3 border border-base-300"
          >
            <li className="menu-title px-4 py-1.5 text-[10px] uppercase font-bold opacity-50">
              {t('bulk.ops_title')}
            </li>
            <li>
              <button className="py-2" onClick={onUpdateInfo}>
                <Edit size={16} className="opacity-70" />
                {t('bulk.edit_metadata')}
              </button>
            </li>
            <li>
              <button className="py-2" onClick={() => onPin(false)}>
                <PinOff size={16} className="opacity-70" />
                {t('bulk.unpin_title')}
              </button>
            </li>
            <li>
              <button className="py-2" onClick={() => onFavorite(false)}>
                <StarOff size={16} className="opacity-70" />
                {t('bulk.unfav_title')}
              </button>
            </li>
            <div className="divider my-0 opacity-50"></div>
            <li>
              <button className="py-2 text-error hover:bg-error/10" onClick={onDelete}>
                <Trash2 size={16} className="opacity-70" />
                {t('bulk.move_trash')}
              </button>
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
