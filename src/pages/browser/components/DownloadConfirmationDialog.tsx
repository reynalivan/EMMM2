import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { DownloadConfirmationRequest } from '../types';

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
  const confirmButtonRef = useRef<HTMLButtonElement>(null);

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
      <div className="modal-box max-w-lg">
        <h2 id="download-confirmation-title" className="text-lg font-bold">
          {t('downloads.confirmation.title')}
        </h2>
        <p id="download-confirmation-description" className="mt-2 text-sm text-base-content/75">
          {t('downloads.confirmation.description')}
        </p>

        <dl className="mt-5 space-y-3 rounded-box bg-base-200 p-4 text-sm">
          <div>
            <dt className="font-medium text-base-content/70">
              {t('downloads.confirmation.filename')}
            </dt>
            <dd className="mt-1 break-all text-base-content">{request.filename}</dd>
          </div>
          <div>
            <dt className="font-medium text-base-content/70">
              {t('downloads.confirmation.source')}
            </dt>
            <dd className="mt-1 break-all text-base-content">{request.source_url}</dd>
          </div>
          <div>
            <dt className="font-medium text-base-content/70">
              {t('downloads.confirmation.destination')}
            </dt>
            <dd className="mt-1 break-all text-base-content">{request.destination_path}</dd>
          </div>
        </dl>

        <div className="modal-action">
          <button
            className="btn btn-ghost"
            type="button"
            onClick={onReject}
            disabled={isSubmitting}
          >
            {t('common:action.cancel')}
          </button>
          <button
            ref={confirmButtonRef}
            className="btn btn-primary"
            type="button"
            onClick={onConfirm}
            disabled={isSubmitting}
          >
            {isSubmitting ? <span className="loading loading-spinner loading-sm" /> : null}
            {t('downloads.confirmation.confirm')}
          </button>
        </div>
      </div>
    </dialog>
  );
}
