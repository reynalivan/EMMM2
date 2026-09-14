import { X } from 'lucide-react';
import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import { LiquidSurface } from '@/shared/ui/liquid';

interface BrowserImagePreviewDialogProps {
  imageUrl: string | null;
  onClose: () => void;
}

export function BrowserImagePreviewDialog({ imageUrl, onClose }: BrowserImagePreviewDialogProps) {
  const { t } = useTranslation(['browser', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);

  useDialogSync(dialogRef, imageUrl !== null);

  return (
    <dialog
      ref={dialogRef}
      className="modal bg-overlay-mask p-0 sm:p-6"
      aria-labelledby="browser-image-preview-title"
      onCancel={onClose}
      onClose={onClose}
    >
      <LiquidSurface
        liquidRole="overlay"
        className="modal-box h-full max-h-none w-full max-w-none rounded-none p-0 shadow-2xl sm:h-[calc(100dvh-3rem)] sm:w-[calc(100dvw-3rem)] sm:rounded-box"
        contentClassName="flex h-full min-h-0 flex-col"
      >
        <header className="flex shrink-0 items-center justify-between border-b border-base-200 px-4 py-3">
          <h2 id="browser-image-preview-title" className="text-base font-semibold">
            {t('image_preview.title')}
          </h2>
          <button
            type="button"
            className="btn btn-ghost btn-sm btn-square"
            onClick={onClose}
            aria-label={t('common:actions.close')}
          >
            <X size={18} />
          </button>
        </header>
        <div className="min-h-0 flex-1 overflow-auto p-4">
          {imageUrl && (
            <img
              src={imageUrl}
              alt={t('image_preview.image_alt')}
              className="mx-auto h-auto max-w-none"
            />
          )}
        </div>
      </LiquidSurface>
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={onClose} aria-label={t('common:actions.close')} />
      </form>
    </dialog>
  );
}
