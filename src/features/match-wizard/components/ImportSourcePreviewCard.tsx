import { useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { File, Folder, Image } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { ImportItem, ImportSourcePreview } from '../../../shared/api/tauri/bindings.gen';

type Props = {
  item: ImportItem;
  loadPreview?: (item: ImportItem) => Promise<ImportSourcePreview>;
  children: ReactNode;
};

export function ImportSourcePreviewCard({ children, item, loadPreview }: Props) {
  const { t } = useTranslation('match_wizard');
  const timer = useRef<number | null>(null);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [preview, setPreview] = useState<ImportSourcePreview | null>(null);
  const [position, setPosition] = useState({ left: 24, top: 24 });

  const show = (element: HTMLElement) => {
    const rect = element.getBoundingClientRect();
    const cardWidth = 384;
    const cardHeight = 420;
    const gap = 12;
    const fitsRight = window.innerWidth - rect.right >= cardWidth + gap;
    setPosition({
      left: fitsRight ? rect.right + gap : Math.max(12, rect.left - cardWidth - gap),
      top: Math.max(12, Math.min(rect.top, window.innerHeight - cardHeight - 12)),
    });
    timer.current = window.setTimeout(() => {
      setOpen(true);
      if (!preview && loadPreview) {
        setLoading(true);
        void loadPreview(item)
          .then(setPreview)
          .catch(() => setPreview(null))
          .finally(() => setLoading(false));
      }
    }, 300);
  };

  const hide = () => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    setOpen(false);
  };

  return (
    <div
      className="min-w-0"
      onMouseEnter={(event) => show(event.currentTarget)}
      onMouseLeave={hide}
      onFocus={(event) => show(event.currentTarget)}
      onBlur={hide}
    >
      {children}
      {open &&
        createPortal(
          <aside
            className="fixed z-[1100] w-96 rounded-xl border border-base-300 bg-base-100 p-3 shadow-2xl"
            style={position}
          >
            <div className="flex gap-3">
              {preview?.thumbnailPath ? (
                <img
                  src={convertFileSrc(preview.thumbnailPath)}
                  alt=""
                  className="h-24 w-28 rounded-lg bg-base-300 object-cover"
                />
              ) : (
                <span className="grid h-24 w-28 place-items-center rounded-lg bg-base-300 text-base-content/25">
                  <Image size={28} aria-hidden="true" />
                </span>
              )}
              <div className="min-w-0 flex-1">
                <h4 className="truncate font-semibold">{item.plannedName}</h4>
                {loading ? (
                  <span className="loading loading-spinner loading-sm mt-5 text-primary" />
                ) : preview ? (
                  <p className="mt-2 text-xs text-base-content/55">
                    {t('source_preview.summary', {
                      folders: preview.folderCount,
                      files: preview.fileCount,
                      size: formatBytes(preview.totalSizeBytes),
                    })}
                  </p>
                ) : (
                  <p className="mt-2 text-xs text-base-content/55">
                    {t('source_preview.unavailable')}
                  </p>
                )}
              </div>
            </div>

            {preview && (
              <>
                <div className="mt-3 max-h-40 overflow-hidden rounded-lg bg-base-200/70 p-2">
                  {preview.entries.slice(0, 12).map((entry) => (
                    <div
                      key={`${entry.kind}:${entry.relativePath}`}
                      className="flex items-center gap-1.5 truncate py-0.5 text-xs text-base-content/65"
                      style={{ paddingLeft: `${entry.depth * 10}px` }}
                    >
                      {entry.kind === 'folder' ? <Folder size={12} /> : <File size={12} />}
                      <span className="truncate">{entry.relativePath}</span>
                    </div>
                  ))}
                  {preview.truncated && (
                    <p className="pt-1 text-[11px] text-base-content/40">
                      {t('source_preview.more')}
                    </p>
                  )}
                </div>
                {preview.imageThumbnails.length > 0 && (
                  <div className="mt-3 flex gap-2 overflow-hidden">
                    {preview.imageThumbnails.map((path) => (
                      <img
                        key={path}
                        src={convertFileSrc(path)}
                        alt=""
                        className="h-14 w-16 rounded-md bg-base-300 object-cover"
                      />
                    ))}
                  </div>
                )}
              </>
            )}
          </aside>,
          document.body,
        )}
    </div>
  );
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(1)} GB`;
}
