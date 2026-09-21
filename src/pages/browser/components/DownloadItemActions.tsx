import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { Ellipsis, FilePenLine, FileText, FolderOpen, ListX, Trash2 } from 'lucide-react';
import type { BrowserDownloadItem } from '../types';
import ConfirmDialog from '@/shared/ui/components/ui/ConfirmDialog';

interface DownloadItemActionsProps {
  item: BrowserDownloadItem;
  onRemoveFromList: () => void;
  onDeleteFile: () => void;
  onRename: (filename: string) => Promise<unknown>;
  onOpenFile: () => void;
  onOpenLocation: () => void;
}

const terminalStatuses = new Set<BrowserDownloadItem['status']>([
  'finished',
  'failed',
  'canceled',
  'imported',
]);

const MENU_ANCHOR_GAP_PX = 4;
const MENU_VIEWPORT_GUTTER_PX = 8;

interface MenuPosition {
  left: number;
  top: number;
}

export function DownloadItemActions({
  item,
  onRemoveFromList,
  onDeleteFile,
  onRename,
  onOpenFile,
  onOpenLocation,
}: DownloadItemActionsProps) {
  const { t } = useTranslation(['browser', 'common']);
  const actionRootRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState<MenuPosition | null>(null);
  const [renameOpen, setRenameOpen] = useState(false);
  const [filename, setFilename] = useState(item.filename);
  const [isRenaming, setIsRenaming] = useState(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

  const isTerminal = terminalStatuses.has(item.status);
  const hasLocalFile = isTerminal && Boolean(item.file_path?.trim());
  const menuId = `download-actions-${item.id}`;
  const renameDialogId = `download-rename-${item.id}`;

  useEffect(() => {
    if (!menuOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (actionRootRef.current?.contains(target) || menuRef.current?.contains(target)) return;

      setMenuOpen(false);
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        setMenuOpen(false);
        menuButtonRef.current?.focus();
        return;
      }
      if (!['ArrowDown', 'ArrowUp'].includes(event.key)) return;

      const items = menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]');
      if (!items?.length) return;
      event.preventDefault();
      const currentIndex = Array.from(items).indexOf(document.activeElement as HTMLButtonElement);
      const nextIndex =
        event.key === 'ArrowDown'
          ? (currentIndex + 1) % items.length
          : (currentIndex - 1 + items.length) % items.length;
      items[nextIndex]?.focus();
    };

    document.addEventListener('pointerdown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    const firstItem = menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]');
    firstItem?.focus();
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [menuOpen]);

  useLayoutEffect(() => {
    if (!menuOpen) {
      setMenuPosition(null);
      return;
    }

    const updateMenuPosition = () => {
      const buttonRect = menuButtonRef.current?.getBoundingClientRect();
      const menuRect = menuRef.current?.getBoundingClientRect();
      if (!buttonRect || !menuRect) return;

      const maxLeft = Math.max(
        MENU_VIEWPORT_GUTTER_PX,
        window.innerWidth - menuRect.width - MENU_VIEWPORT_GUTTER_PX,
      );
      const left = Math.min(
        Math.max(MENU_VIEWPORT_GUTTER_PX, buttonRect.right - menuRect.width),
        maxLeft,
      );
      const belowTop = buttonRect.bottom + MENU_ANCHOR_GAP_PX;
      const aboveTop = buttonRect.top - menuRect.height - MENU_ANCHOR_GAP_PX;
      const top =
        belowTop + menuRect.height <= window.innerHeight - MENU_VIEWPORT_GUTTER_PX ||
        aboveTop < MENU_VIEWPORT_GUTTER_PX
          ? belowTop
          : aboveTop;

      setMenuPosition({ left, top });
    };

    updateMenuPosition();
    window.addEventListener('resize', updateMenuPosition);
    window.addEventListener('scroll', updateMenuPosition, true);
    return () => {
      window.removeEventListener('resize', updateMenuPosition);
      window.removeEventListener('scroll', updateMenuPosition, true);
    };
  }, [menuOpen]);

  useEffect(() => {
    if (!renameOpen) return;
    setFilename(item.filename);
  }, [item.filename, renameOpen]);

  const closeMenu = () => setMenuOpen(false);

  const handleRename = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const nextFilename = filename.trim();
    if (!nextFilename) return;

    setIsRenaming(true);
    try {
      await onRename(nextFilename);
      setRenameOpen(false);
    } finally {
      setIsRenaming(false);
    }
  };

  const renderMenuItem = (
    label: string,
    Icon: typeof FileText,
    onClick: () => void,
    className = '',
  ) => (
    <button
      type="button"
      role="menuitem"
      className={`flex w-full items-center gap-2 rounded-btn px-3 py-2 text-left text-sm hover:bg-base-300 focus-visible:bg-base-300 focus-visible:outline-none ${className}`}
      onClick={() => {
        closeMenu();
        onClick();
      }}
    >
      <Icon size={15} aria-hidden="true" />
      <span>{label}</span>
    </button>
  );

  return (
    <>
      {isTerminal && (
        <div ref={actionRootRef} className="relative">
          <button
            ref={menuButtonRef}
            type="button"
            className="btn btn-ghost btn-xs btn-square"
            aria-label={t('downloads.actions_menu')}
            aria-controls={menuId}
            aria-expanded={menuOpen}
            aria-haspopup="menu"
            title={t('downloads.actions_menu')}
            onClick={() => setMenuOpen((open) => !open)}
          >
            <Ellipsis size={16} aria-hidden="true" />
          </button>
        </div>
      )}

      {menuOpen &&
        createPortal(
          <div
            ref={menuRef}
            id={menuId}
            role="menu"
            aria-label={t('downloads.actions_menu')}
            className="fixed z-50 w-56 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
            style={
              menuPosition
                ? { left: menuPosition.left, top: menuPosition.top }
                : { left: 0, top: 0, visibility: 'hidden' }
            }
          >
            {hasLocalFile && renderMenuItem(t('downloads.open_file'), FileText, onOpenFile)}
            {hasLocalFile &&
              renderMenuItem(t('downloads.open_in_file_explorer'), FolderOpen, onOpenLocation)}
            {hasLocalFile &&
              renderMenuItem(t('downloads.rename'), FilePenLine, () => {
                setFilename(item.filename);
                setRenameOpen(true);
              })}
            {hasLocalFile && <div className="my-1 border-t border-base-300" role="separator" />}
            {renderMenuItem(t('downloads.remove_from_list'), ListX, onRemoveFromList)}
            {renderMenuItem(
              t('downloads.delete_file'),
              Trash2,
              () => setDeleteConfirmOpen(true),
              'text-error hover:text-error',
            )}
          </div>,
          document.body,
        )}

      {renameOpen && (
        <dialog
          open
          className="modal modal-bottom sm:modal-middle bg-overlay-mask z-[100]"
          aria-labelledby={`${renameDialogId}-title`}
          onCancel={() => setRenameOpen(false)}
        >
          <div className="modal-box max-w-md border border-base-content/10 bg-base-100 shadow-2xl">
            <form onSubmit={handleRename}>
              <h2 id={`${renameDialogId}-title`} className="text-base font-semibold">
                {t('downloads.rename_title')}
              </h2>
              <p className="mt-1 text-sm text-base-content/65">
                {t('downloads.rename_description')}
              </p>
              <label className="form-control mt-4 gap-1.5">
                <span className="label-text text-sm">{t('downloads.filename')}</span>
                <input
                  className="input input-bordered w-full"
                  value={filename}
                  onChange={(event) => setFilename(event.target.value)}
                  autoComplete="off"
                  autoFocus
                  disabled={isRenaming}
                />
              </label>
              <div className="modal-action">
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => setRenameOpen(false)}
                  disabled={isRenaming}
                >
                  {t('common:actions.cancel')}
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={isRenaming || !filename.trim()}
                >
                  {isRenaming ? t('downloads.renaming') : t('downloads.save_rename')}
                </button>
              </div>
            </form>
          </div>
          <form method="dialog" className="modal-backdrop">
            <button type="button" onClick={() => setRenameOpen(false)}>
              {t('common:actions.close')}
            </button>
          </form>
        </dialog>
      )}

      <ConfirmDialog
        open={deleteConfirmOpen}
        title={t('downloads.delete_file_title')}
        message={t('downloads.delete_file_confirm', { filename: item.filename })}
        confirmLabel={t('downloads.delete_file')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onConfirm={() => {
          setDeleteConfirmOpen(false);
          onDeleteFile();
        }}
        onCancel={() => setDeleteConfirmOpen(false)}
      />
    </>
  );
}
