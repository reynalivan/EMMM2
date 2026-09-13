import {
  type Dispatch,
  type SetStateAction,
  useEffect,
  useMemo,
  useRef,
  useState,
  forwardRef,
} from 'react';
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
  const runAction = (action: () => void) => {
    onOpenChange(false);
    action();
  };
  const actionClassName =
    'flex w-full items-center gap-2 whitespace-nowrap rounded-md px-3 py-2 text-left text-sm text-base-content transition-colors hover:bg-base-content/8 focus-visible:bg-base-content/8 disabled:pointer-events-none disabled:opacity-40';

  return (
    <details
      className="dropdown dropdown-end absolute right-2 top-2 z-10"
      open={isOpen}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
      onBlur={(event) => {
        const nextFocus = event.relatedTarget;
        if (!(nextFocus instanceof Node) || !event.currentTarget.contains(nextFocus)) {
          onOpenChange(false);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          onOpenChange(false);
        }
      }}
    >
      <summary
        className="glass-surface btn btn-circle btn-sm list-none text-base-content shadow-sm hover:border-primary hover:bg-primary hover:text-primary-content"
        aria-label={t('preview:gallery.actions_label')}
        title={t('preview:gallery.actions_label')}
      >
        <Ellipsis size={18} />
      </summary>
      <LiquidSurface
        liquidRole="overlay"
        className="dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-72 rounded-lg shadow-xl"
      >
        <ul className="menu w-full p-1" aria-label={t('preview:gallery.actions_label')}>
          <li>
            <button
              type="button"
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
    </details>
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

        {images.map((imagePath, index) => {
          const shouldLoad = shouldLoadGalleryImage(index, boundedIndex, images.length);
          const isActive = index === boundedIndex;
          const isBroken = brokenPaths.has(imagePath);

          return (
            <div
              key={imagePath}
              className={`absolute inset-0 transition-opacity ${isActive ? 'opacity-100' : 'pointer-events-none opacity-0'}`}
            >
              {shouldLoad && !isBroken ? (
                <img
                  src={getFileUrl(imagePath)}
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
                  {shouldLoad && isBroken
                    ? t('preview:gallery.broken_image')
                    : t('preview:gallery.image_placeholder')}
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
              window.open(getFileUrl(activePath), '_blank', 'noopener,noreferrer');
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

  useEffect(() => {
    setBrokenPaths((previous) => (previous.size === 0 ? previous : new Set()));
  }, [imageRefreshKey]);

  const activePath = useMemo(() => {
    if (!hasImages) {
      return null;
    }
    return images[boundedIndex] ?? null;
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
          {images.map((imagePath, index) => (
            <button
              key={`${imagePath}-dot`}
              type="button"
              aria-label={t('preview:gallery.go_to_image', { index: index + 1 })}
              className="group flex h-10 w-10 items-center justify-center rounded-full focus-visible:outline-none"
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
          ))}
        </div>
      )}
    </div>
  );
}
