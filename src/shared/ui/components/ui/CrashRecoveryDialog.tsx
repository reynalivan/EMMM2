import { AlertTriangle } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '@/shared/api/tauri/bindings';
import type { PendingCrashReportSummary } from '@/shared/api/tauri/bindings.gen';
import { sendVoluntaryFrontendDiagnostic, type FrontendDiagnostic } from '@/shared/lib/telemetry';

export function CrashRecoveryDialog() {
  const { t } = useTranslation('common');
  const [report, setReport] = useState<PendingCrashReportSummary | null>(null);
  const [deliveryUnavailable, setDeliveryUnavailable] = useState(false);

  useEffect(() => {
    void commands
      .getPendingCrashReport()
      .then(setReport)
      .catch(() => undefined);
  }, []);

  const discard = () => {
    void commands.discardPendingCrashReport();
    setReport(null);
  };

  const send = () => {
    if (!report) return;
    const diagnostic: FrontendDiagnostic = {
      operation: 'app_bootstrap',
      errorCode: report.error_code,
      fingerprint: 'native-abnormal-exit',
    };
    if (!sendVoluntaryFrontendDiagnostic(diagnostic)) {
      setDeliveryUnavailable(true);
      return;
    }
    discard();
  };

  if (!report) return null;

  return (
    <dialog
      open
      className="modal modal-open bg-overlay-mask"
      aria-modal="true"
      aria-labelledby="crash-report-title"
    >
      <section className="modal-box max-w-lg">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 shrink-0 text-warning" size={20} />
          <div>
            <h2 id="crash-report-title" className="text-base font-semibold">
              {t('diagnostics.crash.title')}
            </h2>
            <p className="mt-2 text-sm leading-6 text-base-content/75">
              {t('diagnostics.crash.description')}
            </p>
            {deliveryUnavailable && (
              <p className="mt-3 text-xs text-warning" role="status">
                {t('diagnostics.report.unavailable')}
              </p>
            )}
          </div>
        </div>
        <div className="modal-action">
          <button type="button" className="btn btn-ghost" onClick={discard}>
            {t('diagnostics.crash.dismiss')}
          </button>
          <button type="button" className="btn btn-primary" onClick={send}>
            {t('diagnostics.crash.send')}
          </button>
        </div>
      </section>
    </dialog>
  );
}
