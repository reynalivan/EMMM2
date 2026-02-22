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
} from 'lucide-react';
import { ContextMenuItem, ContextMenuSeparator } from '../../components/ui/ContextMenu';
import { usePasteThumbnail } from '../../hooks/useFolders';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import type { ModFolder } from '../../types/mod';

interface FolderCardContextMenuProps {
  folder: ModFolder;
  onRename: () => void;
  onDelete: () => void;
  onToggle: () => void;
  onToggleFavorite: () => void;
  onEnableOnlyThis?: () => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onNavigate?: (folderName: string) => void;
}

export default function FolderCardContextMenu({
  folder,
  onRename,
  onDelete,
  onToggle,
  onToggleFavorite,
  onEnableOnlyThis,
  onOpenMoveDialog,
  onNavigate,
}: FolderCardContextMenuProps) {
  const pasteThumbnail = usePasteThumbnail();

  const handleOpenExplorer = async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    invoke('open_in_explorer', { path: folder.path }).catch(console.error);
  };

  const handlePasteThumbnail = async () => {
    try {
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

  return (
    <>
      <ContextMenuItem icon={ExternalLink} onClick={handleOpenExplorer}>
        Open in Explorer
      </ContextMenuItem>
      <ContextMenuItem icon={Pencil} onClick={onRename}>
        Rename
      </ContextMenuItem>
      <ContextMenuItem icon={ToggleLeft} onClick={onToggle}>
        {folder.is_enabled ? 'Disable' : 'Enable'}
      </ContextMenuItem>
      {!folder.is_enabled && onEnableOnlyThis && (
        <ContextMenuItem icon={Zap} onClick={onEnableOnlyThis}>
          Enable Only This
        </ContextMenuItem>
      )}
      <ContextMenuItem icon={Star} onClick={onToggleFavorite}>
        {folder.is_favorite ? 'Unfavorite' : 'Favorite'}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon={Pencil} onClick={handlePasteThumbnail}>
        Paste Thumbnail
      </ContextMenuItem>
      <ContextMenuItem icon={ImageIcon} onClick={handleImportThumbnail}>
        Import Thumbnail...
      </ContextMenuItem>
      {onOpenMoveDialog && (
        <>
          <ContextMenuSeparator />
          <ContextMenuItem icon={ArrowRightLeft} onClick={() => onOpenMoveDialog(folder)}>
            Move to Object...
          </ContextMenuItem>
        </>
      )}
      {folder.node_type === 'ModPackRoot' && onNavigate && (
        <>
          <ContextMenuSeparator />
          <ContextMenuItem icon={FolderOpen} onClick={() => onNavigate(folder.folder_name)}>
            Open content mods (Advanced)
          </ContextMenuItem>
        </>
      )}
      <ContextMenuSeparator />
      <ContextMenuItem icon={Trash2} danger onClick={onDelete}>
        Delete to Trash
      </ContextMenuItem>
    </>
  );
}
