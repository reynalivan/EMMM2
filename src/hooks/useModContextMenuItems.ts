import {
  Star,
  ExternalLink,
  Pencil,
  Trash2,
  ToggleLeft,
  Zap,
  Image as ImageIcon,
  ArrowRightLeft,
  FolderOpen,
  ShieldCheck,
  ShieldOff,
  ClipboardPaste,
  RefreshCw,
  type LucideIcon,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '../types/mod';
import { usePasteThumbnail } from './useFolders';
import { commands } from '../lib/bindings';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import { useActiveGame } from './useActiveGame';

export interface ContextMenuItemConfig {
  id: string;
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  danger?: boolean;
  separatorBefore?: boolean;
  hidden?: boolean;
}

export interface UseModContextMenuItemsProps {
  folder: ModFolder;
  onRename: () => void;
  onDelete: () => void;
  onToggleEnabled: () => void;
  onToggleFavorite: () => void;
  onEnableOnlyThis?: () => void;
  onToggleSafe?: () => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onNavigateModPack?: (folderName: string) => void;
  onSyncWithDb?: () => void;
}

export function useModContextMenuItems({
  folder,
  onRename,
  onDelete,
  onToggleEnabled,
  onToggleFavorite,
  onEnableOnlyThis,
  onToggleSafe,
  onOpenMoveDialog,
  onNavigateModPack,
  onSyncWithDb,
}: UseModContextMenuItemsProps): ContextMenuItemConfig[] {
  const { t } = useTranslation(['grid']);
  const { activeGame } = useActiveGame();
  const pasteThumbnail = usePasteThumbnail();

  const handleOpenExplorer = async () => {
    if (!activeGame?.id) return;
    try {
      await commands.openInExplorer({ gameId: activeGame.id, path: folder.path });
    } catch (err) {
      console.error('Failed to open explorer:', err);
    }
  };

  const handlePasteThumbnail = async () => {
    try {
      if (!navigator.clipboard?.read) {
        alert('Clipboard image paste is not supported in this environment.');
        return;
      }
      const items = await navigator.clipboard.read();
      for (const item of items) {
        if (item.types.some((t) => t.startsWith('image/'))) {
          const blob = await item.getType(item.types.find((t) => t.startsWith('image/'))!);
          const buffer = await blob.arrayBuffer();
          const bytes = new Uint8Array(buffer);

          await pasteThumbnail.mutateAsync({
            folderPath: folder.path,
            imageData: Array.from(bytes),
          });
          return; // Only process first image
        }
      }
      alert('No image found in clipboard');
    } catch (err) {
      console.error('Clipboard paste failed:', err);
    }
  };

  const handleImportThumbnail = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });

      if (selected) {
        const contents = await readFile(selected as string);
        await pasteThumbnail.mutateAsync({
          folderPath: folder.path,
          imageData: Array.from(contents),
        });
      }
    } catch (err) {
      console.error('Import failed:', err);
    }
  };

  return [
    {
      id: 'open-explorer',
      label: t('context.open_explorer'),
      icon: ExternalLink,
      onClick: handleOpenExplorer,
    },
    {
      id: 'rename',
      label: t('context.rename'),
      icon: Pencil,
      onClick: onRename,
    },
    {
      id: 'toggle-enabled',
      label: folder.is_enabled ? t('context.disable') : t('context.enable'),
      icon: ToggleLeft,
      onClick: onToggleEnabled,
    },
    {
      id: 'enable-only-this',
      label: t('context.enable_only'),
      icon: Zap,
      onClick: onEnableOnlyThis!,
      hidden: folder.is_enabled || !onEnableOnlyThis,
    },
    {
      id: 'toggle-favorite',
      label: folder.is_favorite ? t('context.unfavorite') : t('context.favorite'),
      icon: Star,
      onClick: onToggleFavorite,
    },
    {
      id: 'paste-thumbnail',
      label: t('context.paste_thumb'),
      icon: ClipboardPaste,
      onClick: handlePasteThumbnail,
      separatorBefore: true,
    },
    {
      id: 'import-thumbnail',
      label: t('context.import_thumb'),
      icon: ImageIcon,
      onClick: handleImportThumbnail,
    },
    {
      id: 'toggle-safe',
      label: folder.is_safe ? t('context.mark_unsafe') : t('context.mark_safe'),
      icon: folder.is_safe ? ShieldOff : ShieldCheck,
      onClick: onToggleSafe!,
      hidden: !onToggleSafe,
      separatorBefore: true,
    },
    {
      id: 'move-to-object',
      label: t('context.move_to_object'),
      icon: ArrowRightLeft,
      onClick: () => onOpenMoveDialog?.(folder),
      hidden: !onOpenMoveDialog,
      separatorBefore: !onToggleSafe, // Add separator if not already added by toggle-safe
    },
    {
      id: 'navigate-mod-pack',
      label: t('context.open_content_mods'),
      icon: FolderOpen,
      onClick: () => onNavigateModPack?.(folder.folder_name),
      hidden: folder.node_type !== 'ModPackRoot' || !onNavigateModPack,
      separatorBefore: true,
    },
    {
      id: 'sync-db',
      label: t('context.sync_db'),
      icon: RefreshCw,
      onClick: onSyncWithDb!,
      hidden: !onSyncWithDb,
      separatorBefore: true,
    },
    {
      id: 'delete',
      label: t('context.delete_trash'),
      icon: Trash2,
      onClick: onDelete,
      danger: true,
      separatorBefore: true,
    },
  ];
}
