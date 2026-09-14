import {
  type Dispatch,
  type SetStateAction,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  forwardRef,
} from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import {
  ChevronLeft,
  ChevronRight,
  ClipboardPaste,
  Ellipsis,
  ImageIcon,
  ImagePlus,
  Loader2,
  Maximize2,
  Trash2,
  X,
} from 'lucide-react';
import { getFileUrl } from '../../../shared/lib/utils';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../../shared/ui/components/ui/ContextMenu';
import { shouldLoadGalleryImage } from '../utils/previewPanelUtils';
import { LiquidSurface } from '@/shared/ui/liquid';

interface GallerySectionProps {
  images: string[];
  imageRefreshKey: number;
  currentImageIndex: number;
  isFetching: boolean;
  canEdit: boolean;
  isMutating: boolean;
  onPrev: () => void;
  onNext: () => void;
  onSelectIndex?: (index: number) => void;
  onPaste: () => void;
  onImport: () => void;
  onRequestRemoveCurrent: () => void;
  onRequestClearAll: () => void;
}

const GalleryMenuContent = ({
  canEdit,
  isMutating,
  hasImages,
  activePath,
  onPaste,
  onImport,
  onRequestRemoveCurrent,
  onRequestClearAll,
}: {
  canEdit: boolean;
  isMutating: boolean;
  hasImages: boolean;
  activePath: string | null;
  onPaste: () => void;
  onImport: () => void;
  onRequestRemoveCurrent: () => void;
  onRequestClearAll: () => void;
}) => {
  const { t } = useTranslation(['preview']);
  return (
    <>
      <ContextMenuItem icon={ClipboardPaste} onClick={onPaste} disabled={!canEdit || isMutating}>
        {t('preview:gallery.menu.paste')}
      </ContextMenuItem>
      <ContextMenuItem icon={ImagePlus} onClick={onImport} disabled={!canEdit || isMutating}>
        {t('preview:gallery.menu.import')}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem
        icon={Trash2}
        danger
        onClick={onRequestRemoveCurrent}
        disabled={!activePath || isMutating}
      >
        {t('preview:gallery.menu.remove_current')}
      </ContextMenuItem>
      <ContextMenuItem
        icon={Trash2}
        danger
        onClick={onRequestClearAll}
        disabled={!hasImages || isMutating}
      >
        {t('preview:gallery.menu.clear_all')}
      </ContextMenuItem>
    </>
  );
};

interface GalleryActionMenuProps {
  canEdit: boolean;
  isMutating: boolean;
  hasImages: boolean;
  activePath: string | null;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  onPaste: () => void;
  onImport: () => void;
  onRequestRemoveCurrent: () => void;
  onRequestClearAll: () => void;
}

function GalleryActionMenu({
  canEdit,
  isMutating,
  hasImages,
  activePath,
  isOpen,
  onOpenChange,
  onPaste,
  onImport,
  onRequestRemoveCurrent,
  onRequestClearAll,
}: GalleryActionMenuProps) {
  const { t } = useTranslation(['preview']);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPosition, setMenuPosition] = useState({ top: 0, left: 0 });
  const runAction = (action: () => void) => {
    onOpenChange(false);
    action();
  };
  const actionClassName =
    'flex w-full items-center gap-2 whitespace-nowrap rounded-md px-3 py-2 text-left text-sm text-base-content transition-colors hover:bg-base-content/8 focus-visible:bg-base-content/8 disabled:pointer-events-none disabled:opacity-40';

  useLayoutEffect(() => {
    if (!isOpen) return;

    const updatePosition = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;

      const rect = trigger.getBoundingClientRect();
      const menuWidth = Math.min(288, window.innerWidth - 16);
      setMenuPosition({
        top: rect.bottom + 8,
        left: Math.max(8, Math.min(rect.right - menuWidth, window.innerWidth - menuWidth - 8)),
      });
    };

    updatePosition();
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);
    return () => {
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    const closeOnOutsidePress = (event: PointerEvent) => {
      const target = event.target as Node;
      if (triggerRef.current?.contains(target) || menuRef.current?.contains(target)) return;
      onOpenChange(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onOpenChange(false);
    };

    document.addEventListener('pointerdown', closeOnOutsidePress);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePress);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [isOpen, onOpenChange]);

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="glass-surface btn btn-circle btn-sm absolute right-2 top-2 z-10 text-base-content shadow-sm hover:border-primary hover:bg-primary hover:text-primary-content"
        aria-label={t('preview:gallery.actions_label')}
        title={t('preview:gallery.actions_label')}
        aria-expanded={isOpen}
        onClick={() => onOpenChange(!isOpen)}
      >
        <Ellipsis size={18} />
      </button>
      {isOpen &&
        createPortal(
          <div
            ref={menuRef}
            data-testid="gallery-action-menu"
            className="fixed z-[var(--workspace-layer-popover)]"
            style={{
              top: menuPosition.top,
              left: menuPosition.left,
              width: 'min(18rem, calc(100vw - 1rem))',
            }}
          >
            <LiquidSurface liquidRole="overlay" className="w-full rounded-lg shadow-xl">
              <ul
                role="menu"
                className="menu w-full p-1"
                aria-label={t('preview:gallery.actions_label')}
              >
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    className={actionClassName}
                    disabled={!canEdit || isMutating}
                    onClick={() => runAction(onPaste)}
                  >
                    <ClipboardPaste size={16} className="text-base-content/70" />
                    {t('preview:gallery.menu.paste')}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    className={actionClassName}
                    disabled={!canEdit || isMutating}
                    onClick={() => runAction(onImport)}
                  >
                    <ImagePlus size={16} className="text-base-content/70" />
                    {t('preview:gallery.menu.import')}
                  </button>
                </li>
                <li className="my-1 h-px bg-base-content/10" role="separator" />
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    className={`${actionClassName} text-error hover:bg-error/10 focus-visible:bg-error/10`}
                    disabled={!activePath || isMutating}
                    onClick={() => runAction(onRequestRemoveCurrent)}
                  >
                    <Trash2 size={16} />
                    {t('preview:gallery.menu.remove_current')}
                  </button>
                </li>
                <li>
                  <button
                    type="button"
                    role="menuitem"
                    className={`${actionClassName} text-error hover:bg-error/10 focus-visible:bg-error/10`}
                    disabled={!hasImages || isMutating}
                    onClick={() => runAction(onRequestClearAll)}
                  >
                    <Trash2 size={16} />
                    {t('preview:gallery.menu.clear_all')}
                  </button>
                </li>
              </ul>
            </LiquidSurface>
          </div>,
          document.body,
        )}
    </>
  );
}

const GalleryTrigger = forwardRef<
  HTMLDivElement,
  {
    hasImages: boolean;
    images: string[];
    boundedIndex: number;
    brokenPaths: Set<string>;
    onPrev: () => void;
    onNext: () => void;
    activePath: string | null;
    setBrokenPaths: Dispatch<SetStateAction<Set<string>>>;
    canEdit: boolean;
    isMutating: boolean;
    onOpenActionMenu: () => void;
    onOpenFullscreen: () => void;
  }
>(
  (
    {
      hasImages,
      images,
      boundedIndex,
      brokenPaths,
      onPrev,
      onNext,
      activePath,
      setBrokenPaths,
      canEdit,
      isMutating,
      onOpenActionMenu,
      onOpenFullscreen,
    },
    ref,
  ) => {
    const { t } = useTranslation(['preview', 'common']);
    return (
      <div
        ref={ref}
        className="group relative aspect-square overflow-hidden rounded-lg border border-base-content/5 bg-base-300/50"
      >
        {!hasImages && (
          <div className="flex flex-col h-full items-center justify-center text-xs text-base-content/30 gap-2">
            <ImageIcon size={24} className="opacity-20" />
            <span>{t('preview:gallery.no_preview')}</span>
            <button
              type="button"
              className="btn btn-ghost btn-sm min-h-10 gap-2 whitespace-nowrap text-base-content/70 hover:text-base-content"
              disabled={!canEdit || isMutating}
              onClick={onOpenActionMenu}
            >
              <ImagePlus size={16} />
              {t('preview:gallery.add_preview_image')}
            </button>
          </div>
        )}

        {images
          .map((imagePath, index) => ({ imagePath, index }))
          .filter(({ index }) => shouldLoadGalleryImage(index, boundedIndex, images.length))
          .map(({ imagePath, index }) => {
            const isActive = index === boundedIndex;
            const isBroken = brokenPaths.has(imagePath);
            const imageUrl = imagePath ? getFileUrl(imagePath) : null;

            return (
              <div
                key={imagePath}
                className={`absolute inset-0 transition-opacity ${isActive ? 'opacity-100' : 'pointer-events-none opacity-0'}`}
              >
                {!isBroken && imageUrl ? (
                  <img
                    src={imageUrl}
                    alt={t('preview:gallery.image_alt')}
                    className="h-full w-full object-cover"
                    loading="lazy"
                    onError={() => {
                      setBrokenPaths((prev) => {
                        const next = new Set(prev);
                        next.add(imagePath);
                        return next;
                      });
                    }}
                  />
                ) : (
                  <div className="flex h-full items-center justify-center text-xs text-base-content/30 text-center px-4">
                    {t('preview:gallery.broken_image')}
                  </div>
                )}
              </div>
            );
          })}

        {images.length > 1 && (
          <div className="absolute left-2 right-2 top-1/2 flex -translate-y-1/2 justify-between">
            <button
              type="button"
              aria-label={t('common:actions.prev')}
              onClick={(e) => {
                e.stopPropagation(); // Prevent menu open on nav click
                onPrev();
              }}
              className="glass-surface btn btn-circle h-10 min-h-10 w-10 text-base-content shadow-sm hover:border-primary hover:bg-primary hover:text-primary-content"
            >
              <ChevronLeft size={18} />
            </button>
            <button
              type="button"
              aria-label={t('common:actions.next')}
              onClick={(e) => {
                e.stopPropagation();
                onNext();
              }}
              className="glass-surface btn btn-circle h-10 min-h-10 w-10 text-base-content shadow-sm hover:border-primary hover:bg-primary hover:text-primary-content"
            >
              <ChevronRight size={18} />
            </button>
          </div>
        )}

        {activePath && (
          <button
            aria-label={t('preview:gallery.maximize_label')}
            className="glass-surface btn btn-circle absolute bottom-2 right-2 h-10 min-h-10 w-10 text-base-content shadow-sm hover:border-primary hover:bg-primary hover:text-primary-content"
            title={t('preview:gallery.maximize_label')}
            onClick={(e) => {
              e.stopPropagation();
              onOpenFullscreen();
            }}
          >
            <Maximize2 size={14} />
          </button>
        )}
      </div>
    );
  },
);

GalleryTrigger.displayName = 'GalleryTrigger';

function galleryPageIndexes(current: number, total: number): Array<number | null> {
  if (total <= 7) return Array.from({ length: total }, (_, index) => index);
  if (current <= 3) return [0, 1, 2, 3, 4, null, total - 1];
  if (current >= total - 4) return [0, null, total - 5, total - 4, total - 3, total - 2, total - 1];
  return [0, null, current - 1, current, current + 1, null, total - 1];
}

function GalleryLightbox({
  activePath,
  imageCount,
  onClose,
  onPrev,
  onNext,
}: {
  activePath: string;
  imageCount: number;
  onClose: () => void;
  onPrev: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation(['preview', 'common']);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault();
        onPrev();
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        onNext();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose, onNext, onPrev]);

  return createPortal(
    <dialog
      open
      className="modal modal-open p-4"
      aria-label={t('preview:gallery.fullscreen')}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <div className="relative flex h-full w-full items-center justify-center">
        <img
          src={getFileUrl(activePath)}
          alt={t('preview:gallery.image_alt')}
          className="max-h-[calc(100vh-2rem)] max-w-full rounded-lg object-contain shadow-2xl"
        />
        <button
          type="button"
          className="btn btn-circle btn-sm absolute right-2 top-2"
          onClick={onClose}
          aria-label={t('common:actions.close')}
        >
          <X size={16} />
        </button>
        {imageCount > 1 && (
          <div className="absolute inset-x-2 top-1/2 flex -translate-y-1/2 justify-between">
            <button
              type="button"
              className="btn btn-circle"
              onClick={onPrev}
              aria-label={t('common:actions.prev')}
            >
              <ChevronLeft />
            </button>
            <button
              type="button"
              className="btn btn-circle"
              onClick={onNext}
              aria-label={t('common:actions.next')}
            >
              <ChevronRight />
            </button>
          </div>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={onClose}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>,
    document.body,
  );
}

export default function GallerySection({
  images,
  imageRefreshKey,
  currentImageIndex,
  isFetching,
  canEdit,
  isMutating,
  onPrev,
  onNext,
  onSelectIndex,
  onPaste,
  onImport,
  onRequestRemoveCurrent,
  onRequestClearAll,
}: GallerySectionProps) {
  const { t } = useTranslation(['preview']);
  const hasImages = images.length > 0;
  const boundedIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const [brokenPaths, setBrokenPaths] = useState<Set<string>>(new Set());
  const touchStartXRef = useRef<number | null>(null);
  const [isActionMenuOpen, setIsActionMenuOpen] = useState(false);
  const [isFullscreenOpen, setIsFullscreenOpen] = useState(false);

  useEffect(() => {
    setBrokenPaths((previous) => (previous.size === 0 ? previous : new Set()));
  }, [imageRefreshKey]);

  const activePath = useMemo(() => {
    if (!hasImages) {
      return null;
    }
    return images[boundedIndex] || null;
  }, [hasImages, images, boundedIndex]);

  return (
    <div className="mb-5">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-base-content/80">{t('preview:gallery.title')}</h3>
        <div className="flex items-center gap-2 text-xs tabular-nums text-base-content/45">
          {isFetching && <Loader2 size={14} className="animate-spin" />}
          <span>{hasImages ? `${boundedIndex + 1} / ${images.length}` : '0 / 0'}</span>
        </div>
      </div>

      <div className="relative">
        <ContextMenu
          content={
            <GalleryMenuContent
              canEdit={canEdit}
              isMutating={isMutating}
              hasImages={hasImages}
              activePath={activePath}
              onPaste={onPaste}
              onImport={onImport}
              onRequestRemoveCurrent={onRequestRemoveCurrent}
              onRequestClearAll={onRequestClearAll}
            />
          }
        >
          <div
            role="region"
            aria-label={t('preview:gallery.slider_label')}
            onTouchStart={(event) => {
              touchStartXRef.current = event.changedTouches[0]?.clientX ?? null;
            }}
            onTouchEnd={(event) => {
              const startX = touchStartXRef.current;
              const endX = event.changedTouches[0]?.clientX ?? null;
              touchStartXRef.current = null;
              if (startX === null || endX === null || images.length <= 1) {
                return;
              }
              const delta = endX - startX;
              if (Math.abs(delta) < 30) {
                return;
              }
              if (delta > 0) {
                onPrev();
              } else {
                onNext();
              }
            }}
          >
            <GalleryTrigger
              hasImages={hasImages}
              images={images}
              boundedIndex={boundedIndex}
              brokenPaths={brokenPaths}
              onPrev={onPrev}
              onNext={onNext}
              activePath={activePath}
              setBrokenPaths={setBrokenPaths}
              canEdit={canEdit}
              isMutating={isMutating}
              onOpenActionMenu={() => setIsActionMenuOpen(true)}
              onOpenFullscreen={() => setIsFullscreenOpen(true)}
            />
          </div>
        </ContextMenu>
        <GalleryActionMenu
          canEdit={canEdit}
          isMutating={isMutating}
          hasImages={hasImages}
          activePath={activePath}
          isOpen={isActionMenuOpen}
          onOpenChange={setIsActionMenuOpen}
          onPaste={onPaste}
          onImport={onImport}
          onRequestRemoveCurrent={onRequestRemoveCurrent}
          onRequestClearAll={onRequestClearAll}
        />
      </div>

      {images.length > 1 && (
        <div className="mt-2 flex items-center justify-center gap-1">
          {galleryPageIndexes(boundedIndex, images.length).map((index, position) =>
            index === null ? (
              <span
                key={`ellipsis-${position}`}
                className="w-3 text-center text-sm text-base-content/40"
                aria-hidden="true"
              >
                …
              </span>
            ) : (
              <button
                key={`${images[index]}-dot`}
                type="button"
                aria-label={t('preview:gallery.go_to_image', { index: index + 1 })}
                aria-current={index === boundedIndex ? 'true' : undefined}
                className="group flex h-7 w-7 items-center justify-center rounded-full focus-visible:outline-none"
                onClick={() => onSelectIndex?.(index)}
              >
                <span
                  className={`h-2 w-2 rounded-full transition-colors ${
                    index === boundedIndex
                      ? 'bg-primary shadow-[0_0_8px_var(--color-primary)]'
                      : 'bg-base-content/30 group-hover:bg-base-content/50'
                  }`}
                />
              </button>
            ),
          )}
        </div>
      )}
      {isFullscreenOpen && activePath && (
        <GalleryLightbox
          activePath={activePath}
          imageCount={images.length}
          onClose={() => setIsFullscreenOpen(false)}
          onPrev={onPrev}
          onNext={onNext}
        />
      )}
    </div>
  );
}
