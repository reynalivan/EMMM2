import { type Dispatch, type SetStateAction, useMemo, useRef, useState, forwardRef } from 'react';
import { ClipboardPaste, ImagePlus, Loader2, Maximize2, Trash2 } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  ContextMenu,
  ContextMenuItem,
  ContextMenuSeparator,
} from '../../../components/ui/ContextMenu';
import { shouldLoadGalleryImage } from '../previewPanelUtils';

interface GallerySectionProps {
  images: string[];
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
}) => (
  <>
    <ContextMenuItem icon={ClipboardPaste} onClick={onPaste} disabled={!canEdit || isMutating}>
      Paste Thumbnail
    </ContextMenuItem>
    <ContextMenuItem icon={ImagePlus} onClick={onImport} disabled={!canEdit || isMutating}>
      Import Thumbnail
    </ContextMenuItem>
    <ContextMenuSeparator />
    <ContextMenuItem
      icon={Trash2}
      danger
      onClick={onRequestRemoveCurrent}
      disabled={!activePath || isMutating}
    >
      Remove This Thumbnail
    </ContextMenuItem>
    <ContextMenuItem
      icon={Trash2}
      danger
      onClick={onRequestClearAll}
      disabled={!hasImages || isMutating}
    >
      Clear All Thumbnails
    </ContextMenuItem>
  </>
);

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
  }
>(
  (
    { hasImages, images, boundedIndex, brokenPaths, onPrev, onNext, activePath, setBrokenPaths },
    ref,
  ) => {
    return (
      <div
        ref={ref}
        className="group relative aspect-square overflow-hidden rounded-lg border border-base-content/5 bg-base-300/50"
      >
        {!hasImages && (
          <div className="flex h-full items-center justify-center text-xs text-base-content/30">
            No preview available
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
                  src={convertFileSrc(imagePath)}
                  alt="Mod preview"
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
                <div className="flex h-full items-center justify-center text-xs text-base-content/30">
                  {shouldLoad && isBroken ? 'Broken image' : 'Image placeholder'}
                </div>
              )}
            </div>
          );
        })}

        {images.length > 1 && (
          <div className="absolute left-2 right-2 top-1/2 flex -translate-y-1/2 justify-between opacity-0 transition-opacity group-hover:opacity-100">
            <button
              type="button"
              aria-label="Previous image"
              onClick={(e) => {
                e.stopPropagation(); // Prevent menu open on nav click
                onPrev();
              }}
              className="btn btn-circle btn-xs border-white/10 bg-black/50 text-white hover:border-primary hover:bg-primary"
            >
              ❮
            </button>
            <button
              type="button"
              aria-label="Next image"
              onClick={(e) => {
                e.stopPropagation();
                onNext();
              }}
              className="btn btn-circle btn-xs border-white/10 bg-black/50 text-white hover:border-primary hover:bg-primary"
            >
              ❯
            </button>
          </div>
        )}

        {activePath && (
          <button
            aria-label="Open current image fullscreen"
            className="btn btn-circle btn-xs absolute bottom-2 right-2 border-white/10 bg-black/50 text-white opacity-0 transition-opacity hover:border-primary hover:bg-primary group-hover:opacity-100"
            title="Fullscreen"
            onClick={(e) => {
              e.stopPropagation();
              window.open(convertFileSrc(activePath), '_blank', 'noopener,noreferrer');
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
  const hasImages = images.length > 0;
  const boundedIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const [brokenPaths, setBrokenPaths] = useState<Set<string>>(new Set());
  const touchStartXRef = useRef<number | null>(null);

  const activePath = useMemo(() => {
    if (!hasImages) {
      return null;
    }
    return images[boundedIndex] ?? null;
  }, [hasImages, images, boundedIndex]);

  return (
    <div className="mb-6">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">
          Preview Images
        </h3>
        <div className="flex items-center gap-2 text-[10px] text-base-content/40">
          {isFetching && <Loader2 size={14} className="animate-spin" />}
          <span>{hasImages ? `${boundedIndex + 1} / ${images.length}` : '0 / 0'}</span>
        </div>
      </div>

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
          aria-label="Preview image slider"
          onContextMenuCapture={(event) => {
            // Prevent native WebView context menu from stealing focus/closing Radix in Tauri.
            event.preventDefault();
          }}
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
          />
        </div>
      </ContextMenu>

      {images.length > 1 && (
        <div className="mt-2 flex items-center justify-center gap-1">
          {images.map((imagePath, index) => (
            <button
              key={`${imagePath}-dot`}
              type="button"
              aria-label={`Go to image ${index + 1}`}
              className={`h-2 w-2 rounded-full transition-all ${
                index === boundedIndex
                  ? 'bg-primary shadow-[0_0_8px_var(--color-primary)]'
                  : 'bg-base-content/30 hover:bg-base-content/50'
              }`}
              onClick={() => onSelectIndex?.(index)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
