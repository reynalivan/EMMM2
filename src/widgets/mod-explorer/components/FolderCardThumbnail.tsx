import { AlertTriangle, Copy, Folder, Lock, PowerOff, Star } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FolderTypeChipView } from '../utils/FolderTypeChip';
import type { WorkspaceExplorerNode } from '@/entities/workspace';

interface FolderCardThumbnailProps {
  folder: WorkspaceExplorerNode;
  typeChip: FolderTypeChipView | null;
  thumbnailSrc: string | null;
  thumbLoading: boolean;
  imgLoaded: boolean;
  imgError: boolean;
  isSelected: boolean;
  isHiddenByMask: boolean;
  isLockedByParent: boolean;
  isSwitchChecked: boolean;
  hasConflict: boolean;
  hasNamingConflict: boolean;
  primaryWarningText: string | null;
  mutationsDisabled: boolean;
  onImageError: () => void;
  onImageLoaded: () => void;
  onToggleFavorite: () => void;
  onToggleSelection: (isShift: boolean) => void;
}

export default function FolderCardThumbnail({
  folder,
  typeChip,
  thumbnailSrc,
  thumbLoading,
  imgLoaded,
  imgError,
  isSelected,
  isHiddenByMask,
  isLockedByParent,
  isSwitchChecked,
  hasConflict,
  hasNamingConflict,
  primaryWarningText,
  mutationsDisabled,
  onImageError,
  onImageLoaded,
  onToggleFavorite,
  onToggleSelection,
}: FolderCardThumbnailProps) {
  const { t } = useTranslation(['grid']);

  return (
    <div className="aspect-square bg-base-300/50 relative group overflow-hidden flex items-center justify-center">
      {(thumbLoading || (thumbnailSrc && !imgLoaded && !imgError)) && (
        <div className="absolute inset-0 skeleton bg-base-300" />
      )}

      {thumbnailSrc ? (
        <img
          src={thumbnailSrc}
          alt=""
          decoding="async"
          className={`h-full w-full object-cover transition-opacity duration-150 motion-reduce:transition-none
            ${isSelected ? 'opacity-100' : 'opacity-90 group-hover:opacity-100'}
            ${imgLoaded ? (isSelected ? 'opacity-100' : 'opacity-85 group-hover:opacity-100') : 'opacity-0'}
            ${isHiddenByMask ? 'blur-xl' : ''}
          `}
          draggable={false}
          onError={onImageError}
          onLoad={onImageLoaded}
        />
      ) : (
        <Folder
          size={40}
          className={`transition-colors duration-300
            ${isSelected ? 'text-primary' : 'text-base-content/15 group-hover:text-base-content/30'}
            ${isHiddenByMask ? 'blur-lg' : ''}`}
        />
      )}

      {isLockedByParent && (
        <div
          className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/85 text-warning-content rounded-md z-10 shadow-sm"
          title={t('card.locked_by_parent')}
        >
          <Lock size={10} />
          <span className="text-[9px] font-bold">{t('card.locked_badge')}</span>
        </div>
      )}

      {!isLockedByParent && typeChip && (
        <div
          className={`absolute top-1.5 left-1.5 flex items-center gap-1 rounded-md px-1.5 py-0.5 shadow-sm z-10 ${typeChip.className}`}
        >
          <typeChip.icon size={10} />
          <span className="text-[9px] font-bold uppercase">{typeChip.label}</span>
        </div>
      )}

      {!isSwitchChecked && (
        <div className="absolute inset-0 flex items-center justify-center bg-overlay-mask z-10 pointer-events-none">
          <PowerOff size={24} className="text-base-content/90 drop-shadow-sm" />
        </div>
      )}

      <button
        onClick={(event) => {
          event.stopPropagation();
          onToggleFavorite();
        }}
        className={`absolute top-1.5 right-8 z-10 rounded-full p-1 transition-[color,opacity,transform] duration-150
           ${
             folder.is_favorite
               ? 'text-warning opacity-100'
               : 'text-base-content/45 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100 hover:text-warning'
           }
         `}
        title={t(folder.is_favorite ? 'card.unfavorite' : 'card.favorite')}
        disabled={mutationsDisabled}
      >
        <Star size={16} className={`drop-shadow-sm ${folder.is_favorite ? 'fill-current' : ''}`} />
      </button>

      {folder.is_misplaced && (
        <div
          className="absolute bottom-1.5 right-1.5 p-1 bg-error/90 text-error-content rounded-full z-10 shadow-sm"
          title={t('card.misplaced_title')}
        >
          <span className="text-[10px] font-bold px-1">!</span>
        </div>
      )}

      {(hasNamingConflict || hasConflict) && (
        <div className="absolute bottom-1.5 left-1.5 z-10 flex items-center rounded-md border border-base-content/10 bg-base-100/90 p-0.5 shadow-sm">
          {hasNamingConflict && (
            <div
              role="img"
              aria-label={t('card.name_conflict')}
              className="flex h-5 w-5 items-center justify-center rounded-sm text-warning"
              title={t('card.name_conflict_title')}
            >
              <AlertTriangle size={10} />
            </div>
          )}
          {hasNamingConflict && hasConflict && <span className="h-3 w-px bg-base-content/15" />}
          {hasConflict && (
            <div
              role="img"
              aria-label={t('card.shared_hash')}
              className="flex h-5 w-5 items-center justify-center rounded-sm text-info"
              title={t('card.hash_conflict_title')}
            >
              <Copy size={10} />
            </div>
          )}
        </div>
      )}

      {!hasNamingConflict && !hasConflict && folder.warnings.length > 0 && (
        <div
          className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-error/90 text-error-content rounded-md z-10 shadow-sm"
          title={primaryWarningText || folder.warnings.join('\n') || t('card.corrupt_ini_title')}
        >
          <AlertTriangle size={10} />
          <span className="text-[9px] font-bold uppercase">{t('badges.corrupt')}</span>
        </div>
      )}

      <div
        className={`absolute right-1.5 top-1.5 z-20 transition-opacity duration-150
          ${isSelected ? 'opacity-100' : 'opacity-100 sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100'}`}
      >
        <input
          type="checkbox"
          className="checkbox checkbox-primary border-2 shadow-sm bg-base-100"
          checked={isSelected}
          onChange={(event) => {
            event.stopPropagation();
            const isShift = event.nativeEvent instanceof MouseEvent && event.nativeEvent.shiftKey;
            onToggleSelection(isShift);
          }}
          onClick={(event) => event.stopPropagation()}
        />
      </div>
    </div>
  );
}
