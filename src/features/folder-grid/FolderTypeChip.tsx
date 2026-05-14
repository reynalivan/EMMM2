import { Folder, Layers, Package, type LucideIcon } from 'lucide-react';
import type { TFunction } from 'i18next';
import type { WorkspaceTypeChip } from '../../types/workspace';

export interface FolderTypeChipView {
  label: string;
  className: string;
  icon: LucideIcon;
}

export function getFolderTypeChip(
  typeChip: WorkspaceTypeChip | null,
  t: TFunction,
  variant: 'card' | 'row',
): FolderTypeChipView | null {
  if (typeChip === 'mod_pack') {
    return {
      label: t('card.mod_pack'),
      className: variant === 'card' ? 'bg-info/90 text-info-content' : 'bg-info/20 text-info',
      icon: Package,
    };
  }

  if (typeChip === 'variant') {
    return {
      label: t('card.variants'),
      className:
        variant === 'card'
          ? 'bg-secondary/90 text-secondary-content'
          : 'bg-secondary/20 text-secondary',
      icon: Layers,
    };
  }

  if (typeChip === 'flat_mod') {
    return {
      label: t('card.flat_mod'),
      className:
        variant === 'card'
          ? 'bg-base-300/90 text-base-content/80'
          : 'bg-base-300 text-base-content/70',
      icon: Folder,
    };
  }

  return null;
}
