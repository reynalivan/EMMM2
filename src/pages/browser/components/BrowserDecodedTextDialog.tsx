import { Copy, ExternalLink, X } from 'lucide-react';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';
import { LiquidSurface } from '@/shared/ui/liquid';
import { toast } from '@/shared/ui/toast';

interface BrowserDecodedTextDialogProps {
  decodedText: string | null;
  onOpenLink: (url: string) => void;
  onClose: () => void;
}

function getDecodedHttpUrl(decodedText: string | null): string | null {
  if (!decodedText) return null;

  try {
    const url = new URL(decodedText.trim());
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.toString() : null;
  } catch {
    return null;
  }
}

export function BrowserDecodedTextDialog({
  decodedText,
  onOpenLink,
  onClose,
}: BrowserDecodedTextDialogProps) {
  const { t } = useTranslation(['browser', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [isCopying, setIsCopying] = useState(false);
  const decodedLink = getDecodedHttpUrl(decodedText);

  useDialogSync(dialogRef, decodedText !== null);

  const handleCopy = async () => {
    if (!decodedText || !navigator.clipboard?.writeText) {
      toast.error(t('common:errors.clipboard_not_supported'));
      return;
    }

    setIsCopying(true);
    try {
      await navigator.clipboard.writeText(decodedText);
      toast.success(t('base64_decode.copied'));
    } catch {
      toast.error(t('base64_decode.copy_failed'));
    } finally {
      setIsCopying(false);
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal bg-overlay-mask p-4 sm:p-6"
      aria-labelledby="browser-base64-decode-title"
      onCancel={onClose}
      onClose={onClose}
    >
      <LiquidSurface
        liquidRole="overlay"
        className="modal-box max-w-2xl p-0 shadow-2xl"
        contentClassName="flex max-h-[min(70dvh,40rem)] min-h-0 flex-col"
      >
        <header className="flex shrink-0 items-center justify-between border-b border-base-200 px-4 py-3">
          <h2 id="browser-base64-decode-title" className="text-base font-semibold">
            {t('base64_decode.title')}
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
        <div className="min-h-0 overflow-auto p-4">
          <pre className="m-0 whitespace-pre-wrap break-words font-mono text-sm text-base-content">
            {decodedText}
          </pre>
        </div>
        <footer className="flex shrink-0 items-center justify-end gap-2 border-t border-base-200 px-4 py-3">
          {decodedLink && (
            <button
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={() => onOpenLink(decodedLink)}
            >
              <ExternalLink size={16} />
              {t('base64_decode.open_link')}
            </button>
          )}
          <button
            type="button"
            className="btn btn-primary btn-sm"
            disabled={!decodedText || isCopying}
            onClick={() => void handleCopy()}
          >
            <Copy size={16} />
            {t('base64_decode.copy')}
          </button>
        </footer>
      </LiquidSurface>
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={onClose} aria-label={t('common:actions.close')} />
      </form>
    </dialog>
  );
}
