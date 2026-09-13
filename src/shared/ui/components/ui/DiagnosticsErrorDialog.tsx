import { AlertTriangle } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  sendVoluntaryFrontendDiagnostic,
  subscribeToFrontendDiagnostics,
  type FrontendDiagnostic,
} from '@/shared/lib/telemetry';

export function DiagnosticsErrorDialog() {
  const { t } = useTranslation('common');
  const [diagnostic, setDiagnostic] = useState<FrontendDiagnostic | null>(null);
  const [deliveryUnavailable, setDeliveryUnavailable] = useState(false);

  useEffect(
    () =>
      subscribeToFrontendDiagnostics((next) => {
        setDeliveryUnavailable(false);
        setDiagnostic(next);
      }),
    [],
  );

  if (!diagnostic) return null;

  const close = () => setDiagnostic(null);
  const send = () => {
    if (sendVoluntaryFrontendDiagnostic(diagnostic)) {
      close();
    } else {
      setDeliveryUnavailable(true);
    }
  };

  return (
    <dialog
      open
      className="modal modal-open bg-overlay-mask"
      aria-modal="true"
      aria-labelledby="diagnostic-report-title"
      onCancel={close}
    >
      <section className="modal-box max-w-lg">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 shrink-0 text-warning" size={20} />
          <div>
            <h2 id="diagnostic-report-title" className="text-base font-semibold">
              {t('diagnostics.report.title')}
            </h2>
            <p className="mt-2 text-sm leading-6 text-base-content/75">
              {t('diagnostics.report.description')}
            </p>
            <p className="mt-3 text-xs text-base-content/60">
              {t('diagnostics.report.code', { code: diagnostic.errorCode })}
            </p>
            {deliveryUnavailable && (
              <p className="mt-3 text-xs text-warning" role="status">
                {t('diagnostics.report.unavailable')}
              </p>
            )}
          </div>
        </div>
        <div className="modal-action">
          <button type="button" className="btn btn-ghost" onClick={close}>
            {t('diagnostics.report.dismiss')}
          </button>
          <button type="button" className="btn btn-primary" onClick={send}>
            {t('diagnostics.report.send')}
          </button>
        </div>
      </section>
      <form method="dialog" className="modal-backdrop">
        <button onClick={close}>{t('actions.close')}</button>
      </form>
    </dialog>
  );
}
