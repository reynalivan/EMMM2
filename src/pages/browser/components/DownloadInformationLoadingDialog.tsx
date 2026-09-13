import { AlertCircle, FileSearch, X } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { DownloadInformationFailure, DownloadInformationLoading } from '../types';

interface DownloadInformationLoadingDialogProps {
  request: DownloadInformationLoading;
  failure?: DownloadInformationFailure | null;
  onDismiss: () => void;
}

/** Visible while WebView2 finishes supplying a download's safe file name. */
export function DownloadInformationLoadingDialog({
  request,
  failure = null,
  onDismiss,
}: DownloadInformationLoadingDialogProps) {
  const { t } = useTranslation(['browser']);
  const dismissButtonRef = useRef<HTMLButtonElement>(null);
  const isFailure = failure !== null;

  useEffect(() => {
    if (isFailure) dismissButtonRef.current?.focus();
  }, [isFailure]);

  const titleKey =
    failure?.reason === 'timeout'
      ? 'downloads.preparing.timeout_title'
      : failure?.reason === 'queue_full'
        ? 'downloads.preparing.queue_full_title'
        : failure
          ? 'downloads.preparing.unavailable_title'
          : 'downloads.preparing.title';
  const descriptionKey =
    failure?.reason === 'timeout'
      ? 'downloads.preparing.timeout_description'
      : failure?.reason === 'queue_full'
        ? 'downloads.preparing.queue_full_description'
        : failure
          ? 'downloads.preparing.unavailable_description'
          : 'downloads.preparing.description';

  return (
    <dialog
      open
      className="modal modal-open bg-overlay-mask backdrop-blur-sm z-10000"
      aria-modal="true"
      aria-labelledby="download-information-title"
      aria-describedby="download-information-description"
      onCancel={(event) => {
        event.preventDefault();
        if (isFailure) onDismiss();
      }}
    >
      <div className="modal-box max-w-md border border-base-300 p-5 shadow-xl">
        <div className="flex items-start gap-3">
          <span className="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-base-200 text-primary">
            {isFailure ? (
              <AlertCircle size={19} className="text-error" aria-hidden />
            ) : (
              <FileSearch size={19} aria-hidden />
            )}
          </span>
          <div>
            <h2 id="download-information-title" className="text-lg font-bold">
              {t(titleKey)}
            </h2>
            <p id="download-information-description" className="mt-2 text-sm text-base-content/75">
              {t(descriptionKey)}
            </p>
          </div>
        </div>
        <p className="mt-5 break-all rounded-box bg-base-200 px-3 py-2.5 text-xs text-base-content/65">
          {request.source_url}
        </p>
        {!isFailure && (
          <div className="mt-4 flex items-center gap-2 text-sm text-base-content/60">
            <span className="loading loading-spinner loading-xs text-primary" aria-hidden />
            {t('downloads.preparing.loading')}
          </div>
        )}
        {isFailure && (
          <div className="modal-action mt-5">
            <button
              ref={dismissButtonRef}
              type="button"
              className="btn btn-ghost btn-sm"
              onClick={onDismiss}
            >
              <X size={16} aria-hidden />
              {t('tabs.close')}
            </button>
          </div>
        )}
      </div>
    </dialog>
  );
}
