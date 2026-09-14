import { AlertTriangle, Download, FileText, FolderOutput, Link } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { DownloadConfirmationRequest } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';

interface DownloadConfirmationDialogProps {
  request: DownloadConfirmationRequest;
  isSubmitting: boolean;
  onConfirm: () => void;
  onReject: () => void;
}

export function DownloadConfirmationDialog({
  request,
  isSubmitting,
  onConfirm,
  onReject,
}: DownloadConfirmationDialogProps) {
  const { t } = useTranslation(['browser']);
  const isBlocked = request.risk_level === 'blocked';
  const confirmButtonRef = useRef<HTMLButtonElement>(null);
  let sourceHost = request.source_url;
  try {
    sourceHost = new URL(request.source_url).hostname;
  } catch {
    // Keep the original value visible when a legacy request has an invalid URL.
  }

  useEffect(() => {
    const previousFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const focusFrame = window.requestAnimationFrame(() => confirmButtonRef.current?.focus());

    return () => {
      window.cancelAnimationFrame(focusFrame);
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, []);

  return (
    <dialog
      open
      className="modal modal-open bg-overlay-mask backdrop-blur-sm z-10000"
      aria-modal="true"
      aria-labelledby="download-confirmation-title"
      aria-describedby="download-confirmation-description"
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.preventDefault();
          if (!isSubmitting) onReject();
        }
      }}
      onCancel={(event) => {
        event.preventDefault();
        if (!isSubmitting) onReject();
      }}
    >
      <div className="modal-box max-w-lg border border-base-300 p-5 shadow-xl">
        <div className="flex items-start gap-3">
          <span className="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-base-200 text-primary">
            <Download size={19} aria-hidden />
          </span>
          <div>
            <h2 id="download-confirmation-title" className="text-lg font-bold">
              {t('downloads.confirmation.title')}
            </h2>
            <p id="download-confirmation-description" className="mt-1 text-sm text-base-content/70">
              {t('downloads.confirmation.description')}
            </p>
          </div>
        </div>

        <dl className="mt-5 divide-y divide-base-300 rounded-box border border-base-300 bg-base-200 text-sm">
          <div className="flex gap-3 px-3 py-3">
            <FileText size={16} className="mt-0.5 shrink-0 text-base-content/55" aria-hidden />
            <div className="min-w-0">
              <dt className="text-xs font-medium text-base-content/60">
                {t('downloads.confirmation.filename')}
              </dt>
              <dd className="mt-1 break-all font-medium text-base-content">{request.filename}</dd>
            </div>
          </div>
          {(request.mime_type || request.bytes_total !== undefined) && (
            <div className="grid grid-cols-2 gap-3 px-3 py-3">
              <div className="min-w-0">
                <dt className="text-xs font-medium text-base-content/60">
                  {t('downloads.confirmation.file_type')}
                </dt>
                <dd
                  className="mt-1 truncate text-base-content/80"
                  title={request.mime_type ?? undefined}
                >
                  {request.mime_type || t('downloads.unknown_type')}
                </dd>
              </div>
              <div>
                <dt className="text-xs font-medium text-base-content/60">
                  {t('downloads.confirmation.file_size')}
                </dt>
                <dd className="mt-1 text-base-content/80">
                  {request.bytes_total != null
                    ? formatBytes(request.bytes_total)
                    : t('downloads.unknown_size')}
                </dd>
              </div>
            </div>
          )}
          <div className="flex gap-3 px-3 py-3">
            <Link size={16} className="mt-0.5 shrink-0 text-base-content/55" aria-hidden />
            <div className="min-w-0">
              <dt className="text-xs font-medium text-base-content/60">
                {t('downloads.confirmation.source')}
              </dt>
              <dd
                className="mt-1 truncate font-medium text-base-content"
                title={request.source_url}
              >
                {sourceHost}
              </dd>
              <dd
                className="mt-0.5 truncate text-xs text-base-content/55"
                title={request.source_url}
              >
                {request.source_url}
              </dd>
            </div>
          </div>
          <div className="flex gap-3 px-3 py-3">
            <FolderOutput size={16} className="mt-0.5 shrink-0 text-base-content/55" aria-hidden />
            <div className="min-w-0">
              <dt className="text-xs font-medium text-base-content/60">
                {t('downloads.confirmation.destination')}
              </dt>
              <dd className="mt-1 break-all text-base-content/80">{request.destination_path}</dd>
            </div>
          </div>
        </dl>

        {isBlocked && (
          <p className="mt-3 flex gap-2 rounded-box border border-error/35 bg-error/10 px-3 py-2 text-xs text-base-content/80">
            <AlertTriangle size={16} className="shrink-0 text-error" aria-hidden />
            {t('downloads.confirmation.blocked_file_warning')}
          </p>
        )}

        {request.risk_level === 'warning' && (
          <p className="mt-3 flex gap-2 rounded-box border border-warning/35 bg-warning/10 px-3 py-2 text-xs text-base-content/80">
            <AlertTriangle size={16} className="shrink-0 text-warning" aria-hidden />
            {t('downloads.confirmation.risky_file_warning')}
          </p>
        )}

        <div className="modal-action mt-5">
          <button
            ref={isBlocked ? confirmButtonRef : undefined}
            className="btn btn-ghost"
            type="button"
            onClick={onReject}
            disabled={isSubmitting}
          >
            {t('common:action.cancel')}
          </button>
          {!isBlocked && (
            <button
              ref={isBlocked ? undefined : confirmButtonRef}
              className="btn btn-primary min-w-28"
              type="button"
              onClick={onConfirm}
              disabled={isSubmitting}
              aria-busy={isSubmitting}
            >
              {isSubmitting ? (
                <span className="loading loading-spinner loading-sm" aria-hidden />
              ) : (
                <Download size={16} aria-hidden />
              )}
              {t('downloads.confirmation.confirm')}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
