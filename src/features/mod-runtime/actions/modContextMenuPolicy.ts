import type { TFunction } from 'i18next';
import {
  ArrowRightLeft,
  ClipboardPaste,
  ExternalLink,
  FolderOpen,
  Image as ImageIcon,
  Pencil,
  RefreshCw,
  ShieldCheck,
  ShieldOff,
  Star,
  ToggleLeft,
  Trash2,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import type { WorkspaceExplorerNode } from '../../../types/workspace';

export interface ModContextMenuItemConfig {
  id: string;
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  danger?: boolean;
  separatorBefore?: boolean;
  hidden?: boolean;
}

export interface ModContextMenuActionHandlers {
  openExplorer?: () => void;
  rename: () => void;
  toggleEnabled: () => void;
  enableOnlyThis?: () => void;
  toggleFavorite: () => void;
  pasteThumbnail?: () => void;
  importThumbnail?: () => void;
  toggleSafe?: () => void;
  moveToObject?: () => void;
  navigateModPack?: () => void;
  syncWithDb?: () => void;
  delete: () => void;
}

export function buildModContextMenuItems(
  t: TFunction,
  folder: WorkspaceExplorerNode,
  handlers: ModContextMenuActionHandlers,
): ModContextMenuItemConfig[] {
  return [
    {
      id: 'open-explorer',
      label: t('context.open_explorer'),
      icon: ExternalLink,
      onClick: handlers.openExplorer ?? (() => undefined),
      hidden: !handlers.openExplorer || !folder.capabilities.can_open_in_explorer,
    },
    {
      id: 'rename',
      label: t('context.rename'),
      icon: Pencil,
      onClick: handlers.rename,
      hidden: !folder.capabilities.can_rename,
    },
    {
      id: 'toggle-enabled',
      label: folder.is_enabled ? t('context.disable') : t('context.enable'),
      icon: ToggleLeft,
      onClick: handlers.toggleEnabled,
      hidden: !folder.capabilities.can_toggle,
    },
    {
      id: 'enable-only-this',
      label: t('context.enable_only'),
      icon: Zap,
      onClick: handlers.enableOnlyThis ?? (() => undefined),
      hidden:
        folder.is_enabled ||
        !handlers.enableOnlyThis ||
        !folder.capabilities.can_enable_only_this,
    },
    {
      id: 'toggle-favorite',
      label: folder.is_favorite ? t('context.unfavorite') : t('context.favorite'),
      icon: Star,
      onClick: handlers.toggleFavorite,
      hidden: !folder.capabilities.can_pin,
    },
    {
      id: 'paste-thumbnail',
      label: t('context.paste_thumb'),
      icon: ClipboardPaste,
      onClick: handlers.pasteThumbnail ?? (() => undefined),
      separatorBefore: true,
      hidden: !handlers.pasteThumbnail || !folder.capabilities.can_edit_metadata,
    },
    {
      id: 'import-thumbnail',
      label: t('context.import_thumb'),
      icon: ImageIcon,
      onClick: handlers.importThumbnail ?? (() => undefined),
      hidden: !handlers.importThumbnail || !folder.capabilities.can_edit_metadata,
    },
    {
      id: 'toggle-safe',
      label: folder.is_safe ? t('context.mark_unsafe') : t('context.mark_safe'),
      icon: folder.is_safe ? ShieldOff : ShieldCheck,
      onClick: handlers.toggleSafe ?? (() => undefined),
      hidden: !handlers.toggleSafe || !folder.capabilities.can_toggle_safe,
      separatorBefore: true,
    },
    {
      id: 'move-to-object',
      label: t('context.move_to_object'),
      icon: ArrowRightLeft,
      onClick: handlers.moveToObject ?? (() => undefined),
      hidden: !handlers.moveToObject || !folder.capabilities.can_move,
      separatorBefore: !handlers.toggleSafe,
    },
    {
      id: 'navigate-mod-pack',
      label: t('context.open_content_mods'),
      icon: FolderOpen,
      onClick: handlers.navigateModPack ?? (() => undefined),
      hidden:
        folder.node_type !== 'ModPackRoot' ||
        !handlers.navigateModPack ||
        !folder.can_navigate,
      separatorBefore: true,
    },
    {
      id: 'sync-db',
      label: t('context.sync_db'),
      icon: RefreshCw,
      onClick: handlers.syncWithDb ?? (() => undefined),
      hidden: !handlers.syncWithDb || !folder.capabilities.can_sync,
      separatorBefore: true,
    },
    {
      id: 'delete',
      label: t('context.delete_trash'),
      icon: Trash2,
      onClick: handlers.delete,
      danger: true,
      separatorBefore: true,
      hidden: !folder.capabilities.can_delete,
    },
  ];
}
