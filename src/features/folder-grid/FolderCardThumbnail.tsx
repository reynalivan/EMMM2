import { AlertTriangle, Copy, Folder, Lock, PowerOff, Star } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FolderTypeChipView } from './FolderTypeChip';
import type { WorkspaceExplorerNode } from '../../types/workspace';

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
          className={`w-full h-full object-cover transition-all duration-500
            ${isSelected ? 'scale-105' : 'scale-100 group-hover:scale-105'}
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
        className={`absolute top-1.5 right-8 p-1 rounded-full transition-all duration-200 z-10
           ${
             folder.is_favorite
               ? 'text-warning opacity-100 hover:scale-110'
               : 'text-base-content/20 opacity-0 group-hover:opacity-100 hover:text-warning hover:scale-110'
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
          <div className="w-2 h-2 rounded-full bg-current animate-ping absolute inset-0 opacity-75" />
          <span className="text-[10px] font-bold px-1">!</span>
        </div>
      )}

      {hasConflict && (
        <div
          className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/90 text-warning-content rounded-md z-10 shadow-sm"
          title={t('card.hash_conflict_title')}
        >
          <Copy size={10} />
          <span className="text-[9px] font-bold">{t('card.conflict')}</span>
        </div>
      )}

      {hasNamingConflict && !hasConflict && (
        <div
          className="absolute bottom-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 bg-warning/90 text-warning-content rounded-md z-10 shadow-sm animate-pulse"
          title={t('card.name_conflict_title')}
        >
          <AlertTriangle size={10} />
          <span className="text-[9px] font-bold">{t('card.name_conflict')}</span>
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
        className={`absolute top-1.5 right-1.5 transition-all duration-200 z-20
          ${isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-90 group-hover:opacity-100 group-hover:scale-100'}`}
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
