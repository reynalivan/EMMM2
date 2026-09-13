import React, { useCallback, useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { MoreVertical } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { useModContextMenuItems } from '@/features/mod-runtime';
import { useModContextMenuActions } from '@/features/mod-runtime';
import { LiquidSurface } from '@/shared/ui/liquid';

interface PreviewPanelContextMenuProps {
  folder: WorkspaceExplorerNode;
  onRename: () => void;
  onDelete: () => void;
  onToggle: (folder: ModFolder) => void;
  onToggleFavorite: (folder: ModFolder) => void;
  onEnableOnlyThis: (folder: ModFolder) => void;
  onOpenMoveDialog?: (folder: ModFolder) => void;
  onToggleSafe: (folder: ModFolder) => void;
}

export default function PreviewPanelContextMenu({
  folder,
  onRename,
  onDelete,
  onToggle,
  onToggleFavorite,
  onEnableOnlyThis,
  onOpenMoveDialog,
  onToggleSafe,
}: PreviewPanelContextMenuProps) {
  const { t } = useTranslation('preview');
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuSurfaceRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLUListElement>(null);
  const menuId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const contextActions = useModContextMenuActions(folder);
  const items = useModContextMenuItems({
    folder,
    onRename,
    onDelete,
    onToggleEnabled: () => onToggle(folder),
    onToggleFavorite: () => onToggleFavorite(folder),
    onEnableOnlyThis: () => onEnableOnlyThis(folder),
    onToggleSafe: () => onToggleSafe(folder),
    onOpenMoveDialog,
    onOpenExplorer: contextActions.openExplorer,
    onPasteThumbnail: contextActions.pasteThumbnailFromClipboard,
    onImportThumbnail: contextActions.importThumbnail,
  });

  const updatePosition = useCallback(() => {
    const triggerRect = triggerRef.current?.getBoundingClientRect();
    if (!triggerRect) {
      return;
    }

    const viewportMargin = 8;
    const menuWidth = 224;
    const menuHeight = menuSurfaceRef.current?.offsetHeight ?? 0;
    const preferredLeft = triggerRect.right - menuWidth;
    const left = Math.max(
      viewportMargin,
      Math.min(preferredLeft, window.innerWidth - menuWidth - viewportMargin),
    );
    const spaceBelow = window.innerHeight - triggerRect.bottom - viewportMargin;
    const canOpenAbove = triggerRect.top - viewportMargin >= menuHeight;
    const top =
      menuHeight > 0 && spaceBelow < menuHeight && canOpenAbove
        ? triggerRect.top - menuHeight - viewportMargin
        : triggerRect.bottom + viewportMargin;

    setPosition({ top, left });
  }, []);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const closeOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target;
      if (
        target instanceof Node &&
        !menuSurfaceRef.current?.contains(target) &&
        !triggerRef.current?.contains(target)
      ) {
        setIsOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };

    updatePosition();
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    document.addEventListener('keydown', closeOnEscape);
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    const focusFirstMenuItem = window.requestAnimationFrame(() => {
      menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
    });

    return () => {
      window.cancelAnimationFrame(focusFirstMenuItem);
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      document.removeEventListener('keydown', closeOnEscape);
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  }, [isOpen, updatePosition]);

  const handleMenuKeyDown = (event: React.KeyboardEvent<HTMLUListElement>) => {
    const menuItems = Array.from(
      menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [],
    );
    const currentIndex = menuItems.indexOf(document.activeElement as HTMLButtonElement);

    if (event.key === 'Escape') {
      event.preventDefault();
      setIsOpen(false);
      triggerRef.current?.focus();
      return;
    }

    if (menuItems.length === 0) {
      return;
    }

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      const nextIndex = (currentIndex + direction + menuItems.length) % menuItems.length;
      menuItems[nextIndex]?.focus();
    }

    if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      menuItems[event.key === 'Home' ? 0 : menuItems.length - 1]?.focus();
    }
  };

  const menu = isOpen ? (
    <div
      ref={menuSurfaceRef}
      className="fixed z-[calc(var(--workspace-layer-overlay)+1)] w-56"
      style={position}
    >
      <LiquidSurface
        liquidRole="overlay"
        className="max-h-[calc(100vh-1rem)] w-full rounded-box text-base-content shadow-xl"
      >
        <ul
          ref={menuRef}
          id={menuId}
          role="menu"
          aria-label={t('actions.more_actions')}
          className="menu max-h-[calc(100vh-1rem)] w-full overflow-y-auto p-2"
          onKeyDown={handleMenuKeyDown}
        >
          {items.map((item) => {
            if (item.hidden) return null;

            return (
              <React.Fragment key={item.id}>
                {item.separatorBefore && <li role="separator" className="divider my-0" />}
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    onClick={() => {
                      setIsOpen(false);
                      item.onClick();
                    }}
                    className={item.danger ? 'text-error hover:bg-error/10 hover:text-error' : ''}
                  >
                    <item.icon size={14} className="opacity-70" /> {item.label}
                  </button>
                </li>
              </React.Fragment>
            );
          })}
        </ul>
      </LiquidSurface>
    </div>
  ) : null;

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-base-content hover:bg-base-content/10"
        title={t('actions.more_actions')}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-controls={isOpen ? menuId : undefined}
        onClick={() => setIsOpen((open) => !open)}
      >
        <MoreVertical size={16} />
      </button>
      {menu && createPortal(menu, document.body)}
    </>
  );
}
