import { useEffect, useRef, useState } from 'react';
import { AlertCircle, RefreshCw, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  closeWorkspaceDialog,
  getWorkspaceRuntimeState,
  useWorkspaceRuntimeSelector,
} from '@/features/workspace-runtime/@x/file-watcher';
import { commands, type FileLockProcess, type FileLockReport } from '@/shared/api/tauri/bindings';
import {
  extractFileInUsePayload,
  FileLockDiagnosticError,
  formatAppError,
} from '@/shared/lib/appError';
import { pathBasename } from '@/shared/lib/pathKey';
import { useDialogSync } from '@/shared/lib/hooks/useDialogSync';

export function FileInUseDialog() {
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const data = dialogState.kind === 'fileInUse' ? dialogState.data : null;
  const { t } = useTranslation('common');
  const dialogRef = useRef<HTMLDialogElement>(null);
  const busyRef = useRef(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<FileLockReport | null>(null);
  const [recoveryId, setRecoveryId] = useState<string | undefined>(data?.recoveryId);
  const recoveryIdRef = useRef(data?.recoveryId);
  const [status, setStatus] = useState<string | null>(null);
  const [retryAllowed, setRetryAllowed] = useState(true);
  const [confirmation, setConfirmation] = useState<FileLockProcess | null>(null);

  useDialogSync(dialogRef, data !== null);

  function ownsDialog() {
    const current = getWorkspaceRuntimeState().dialogState;
    return current.kind === 'fileInUse' && current.data === data;
  }

  useEffect(() => {
    let cancelled = false;
    busyRef.current = false;
    setBusy(false);
    setReport(null);
    setRecoveryId(data?.recoveryId);
    recoveryIdRef.current = data?.recoveryId;
    setStatus(null);
    setRetryAllowed(true);
    setConfirmation(null);
    if (data?.recoveryId) {
      busyRef.current = true;
      setBusy(true);
      commands
        .getFileLockInfo(data.recoveryId)
        .then((info) => {
          if (!cancelled) setReport(info);
          if (!cancelled) {
            recoveryIdRef.current = info.recovery_id;
            setRecoveryId(info.recovery_id);
          }
        })
        .catch((error: unknown) => {
          if (!cancelled) setStatus(formatAppError(error));
        })
        .finally(() => {
          if (!cancelled) {
            busyRef.current = false;
            setBusy(false);
          }
        });
    }
    return () => {
      cancelled = true;
    };
  }, [data]);

  if (!data) return null;

  function closeDialog() {
    if (busyRef.current || !ownsDialog()) return;
    const dismissal =
      data?.onDismiss?.() ??
      (recoveryIdRef.current ? commands.dismissFileLockRecovery(recoveryIdRef.current) : null);
    if (dismissal) {
      void dismissal.catch((error: unknown) => {
        console.error('[FileLock] Could not dismiss recovery context:', formatAppError(error));
      });
    }
    closeWorkspaceDialog('fileInUse');
  }

  async function refreshInfo(id = recoveryIdRef.current) {
    if (!id) return;
    const info = await commands.getFileLockInfo(id);
    if (ownsDialog()) {
      setReport(info);
      setRecoveryId(info.recovery_id);
      recoveryIdRef.current = info.recovery_id;
    }
    return info;
  }

  async function runAction(action: () => Promise<void>) {
    if (busyRef.current || !ownsDialog()) return;
    busyRef.current = true;
    setBusy(true);
    setStatus(null);
    try {
      await action();
    } catch (error) {
      if (ownsDialog()) setStatus(formatAppError(error));
    } finally {
      if (ownsDialog()) {
        busyRef.current = false;
        setBusy(false);
      }
    }
  }

  async function retryOperation() {
    if (!data?.onRetry || !retryAllowed) return;
    await runAction(async () => {
      const info = recoveryIdRef.current ? await refreshInfo() : undefined;
      if (!ownsDialog()) return;
      try {
        await data.onRetry?.(info?.recovery_id ?? recoveryIdRef.current);
        if (ownsDialog()) {
          busyRef.current = false;
          closeDialog();
        }
      } catch (error) {
        if (!ownsDialog()) return;
        const lock = extractFileInUsePayload(error);
        setStatus(lock ? t('file_in_use.still_locked') : formatAppError(error));
        if (!lock && !(error instanceof FileLockDiagnosticError)) {
          setRetryAllowed(false);
          return;
        }
        if (!lock) return;
        const previousId = recoveryIdRef.current;
        const nextId = lock.recoveryId ?? previousId;
        if (nextId !== previousId) setReport(null);
        if (previousId && nextId !== previousId) {
          void commands.dismissFileLockRecovery(previousId).catch((dismissError: unknown) => {
            console.error(
              '[FileLock] Could not dismiss superseded recovery context:',
              formatAppError(dismissError),
            );
          });
        }
        setRecoveryId(nextId);
        recoveryIdRef.current = nextId;
        try {
          await refreshInfo(nextId);
        } catch (refreshError) {
          if (ownsDialog())
            setStatus(`${t('file_in_use.still_locked')} ${formatAppError(refreshError)}`);
        }
      }
    });
  }

  async function controlProcess(process: FileLockProcess, force: boolean) {
    if (!recoveryId) return;
    setConfirmation(null);
    await runAction(async () => {
      const info = await commands.controlFileLockProcess(recoveryId, process.process_ref, force);
      if (!ownsDialog()) return;
      setReport(info);
      setStatus(info.action_outcome ?? t('file_in_use.process_action_done'));
    });
  }

  const folderName = pathBasename(data.path) || t('file_in_use.folder_fallback');
  const processes = report?.processes ?? [];
  return (
    <dialog
      ref={dialogRef}
      data-testid="file-lock-dialog"
      className="modal modal-bottom sm:modal-middle"
      aria-labelledby="file-in-use-title"
      onCancel={(event) => {
        event.preventDefault();
        closeDialog();
      }}
      onClose={closeDialog}
    >
      <div className="modal-box max-w-xl border border-warning/20 bg-base-200" aria-busy={busy}>
        <div className="flex items-center gap-3 mb-4">
          <AlertCircle className="w-5 h-5 text-warning shrink-0" />
          <h3 id="file-in-use-title" className="text-xl font-bold">
            {t('file_in_use.title')}
          </h3>
        </div>
        <div className="space-y-4">
          <p className="text-sm text-base-content/70">
            {t('file_in_use.description', { folderName })}
          </p>
          <p className="text-xs break-all text-base-content/70">{data.path}</p>
          {processes.length > 0 ? (
            <ul className="bg-base-300 rounded-lg p-3 max-h-56 overflow-y-auto space-y-3">
              {processes.map((process) => (
                <li
                  key={process.process_ref}
                  data-testid={`file-lock-process-${process.pid}`}
                  data-process-pid={process.pid}
                  className="space-y-1 text-sm"
                >
                  <p className="font-medium">
                    {process.name}{' '}
                    <span className="text-base-content/60">
                      {t('file_in_use.process_pid', { pid: process.pid })}
                    </span>
                  </p>
                  {process.executable && (
                    <p className="text-xs break-all text-base-content/60">{process.executable}</p>
                  )}
                  <div className="flex flex-wrap gap-2">
                    <button
                      className="btn btn-sm btn-outline"
                      disabled={busy || !process.can_close || !!confirmation}
                      onClick={() => void controlProcess(process, false)}
                    >
                      {t('file_in_use.close_app')}
                    </button>
                    <button
                      data-testid={`file-lock-force-${process.pid}`}
                      className="btn btn-sm btn-error btn-outline"
                      disabled={busy || !process.can_terminate || !!confirmation}
                      onClick={() => setConfirmation(process)}
                    >
                      {t('file_in_use.force_terminate')}
                    </button>
                  </div>
                  {process.unavailable_reason && (
                    <p className="text-xs text-base-content/60">{process.unavailable_reason}</p>
                  )}
                </li>
              ))}
            </ul>
          ) : !report && data.processes.length > 0 ? (
            <ul className="bg-base-300 rounded-lg p-3 space-y-1 text-sm">
              {data.processes.map((name, index) => (
                <li key={`${name}-${index}`}>
                  {name === 'explorer.exe' ? t('file_in_use.windows_explorer') : name}
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-sm text-base-content/70">{t('file_in_use.unknown_holder')}</p>
          )}
          {report && report.status !== 'complete' && (
            <p className="text-xs text-base-content/70">{t(`file_in_use.scan_${report.status}`)}</p>
          )}
          {report?.message && <p className="text-xs text-base-content/70">{report.message}</p>}
          <div data-testid="file-lock-status" role="status" aria-live="polite" className="text-sm">
            {busy ? t('file_in_use.working') : status}
          </div>
          {!retryAllowed && (
            <p className="text-sm text-warning">{t('file_in_use.retry_blocked')}</p>
          )}
          {confirmation && (
            <div className="rounded-lg border border-error/40 p-3 space-y-3" role="alert">
              <p className="text-sm">
                {t('file_in_use.terminate_confirmation', {
                  name: confirmation.name,
                  pid: confirmation.pid,
                })}
              </p>
              <div className="flex flex-wrap gap-2">
                <button
                  data-testid="file-lock-confirm-terminate"
                  className="btn btn-error btn-sm"
                  disabled={busy}
                  onClick={() => void controlProcess(confirmation, true)}
                >
                  {t('file_in_use.confirm_terminate')}
                </button>
                <button
                  className="btn btn-ghost btn-sm"
                  disabled={busy}
                  onClick={() => setConfirmation(null)}
                >
                  {t('file_in_use.keep_app')}
                </button>
              </div>
            </div>
          )}
          <p className="text-xs text-base-content/60">{t('file_in_use.hint')}</p>
        </div>
        <div className="modal-action flex flex-wrap gap-2">
          <button
            data-testid="file-lock-cancel"
            className="btn btn-ghost btn-sm"
            disabled={busy}
            onClick={closeDialog}
          >
            <X className="w-4 h-4" />
            {t('actions.cancel')}
          </button>
          {recoveryId && (
            <button
              data-testid="file-lock-refresh"
              className="btn btn-outline btn-sm"
              disabled={busy || !!confirmation}
              onClick={() =>
                void runAction(async () => {
                  await refreshInfo();
                })
              }
            >
              {t('file_in_use.refresh_processes')}
            </button>
          )}
          <button
            data-testid="file-lock-retry"
            className="btn btn-warning btn-sm"
            disabled={busy || !data.onRetry || !retryAllowed || !!confirmation}
            onClick={() => void retryOperation()}
          >
            <RefreshCw className="w-4 h-4" />
            {t('actions.retry')}
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-base-300/60 backdrop-blur-sm">
        <button disabled={busy} onClick={closeDialog}>
          {t('actions.close')}
        </button>
      </div>
    </dialog>
  );
}
