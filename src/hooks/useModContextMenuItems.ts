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
  type LucideIcon,
} from 'lucide-react';
import type { ModFolder } from '../types/mod';
import { usePasteThumbnail } from './useFolders';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';

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
}: UseModContextMenuItemsProps): ContextMenuItemConfig[] {
  const pasteThumbnail = usePasteThumbnail();

  const handleOpenExplorer = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('open_in_explorer', { path: folder.path });
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
      label: 'Open in Explorer',
      icon: ExternalLink,
      onClick: handleOpenExplorer,
    },
    {
      id: 'rename',
      label: 'Rename',
      icon: Pencil,
      onClick: onRename,
    },
    {
      id: 'toggle-enabled',
      label: folder.is_enabled ? 'Disable' : 'Enable',
      icon: ToggleLeft,
      onClick: onToggleEnabled,
    },
    {
      id: 'enable-only-this',
      label: 'Enable Only This',
      icon: Zap,
      onClick: onEnableOnlyThis!,
      hidden: folder.is_enabled || !onEnableOnlyThis,
    },
    {
      id: 'toggle-favorite',
      label: folder.is_favorite ? 'Unfavorite' : 'Favorite',
      icon: Star,
      onClick: onToggleFavorite,
    },
    {
      id: 'paste-thumbnail',
      label: 'Paste Thumbnail',
      icon: ClipboardPaste,
      onClick: handlePasteThumbnail,
      separatorBefore: true,
    },
    {
      id: 'import-thumbnail',
      label: 'Import Thumbnail...',
      icon: ImageIcon,
      onClick: handleImportThumbnail,
    },
    {
      id: 'toggle-safe',
      label: folder.is_safe ? 'Mark as Unsafe' : 'Mark as Safe',
      icon: folder.is_safe ? ShieldOff : ShieldCheck,
      onClick: onToggleSafe!,
      hidden: !onToggleSafe,
      separatorBefore: true,
    },
    {
      id: 'move-to-object',
      label: 'Move to Object...',
      icon: ArrowRightLeft,
      onClick: () => onOpenMoveDialog?.(folder),
      hidden: !onOpenMoveDialog,
      separatorBefore: !onToggleSafe, // Add separator if not already added by toggle-safe
    },
    {
      id: 'navigate-mod-pack',
      label: 'Open content mods (Advanced)',
      icon: FolderOpen,
      onClick: () => onNavigateModPack?.(folder.folder_name),
      hidden: folder.node_type !== 'ModPackRoot' || !onNavigateModPack,
      separatorBefore: true,
    },
    {
      id: 'delete',
      label: 'Delete to Trash',
      icon: Trash2,
      onClick: onDelete,
      danger: true,
      separatorBefore: true,
    },
  ];
}
