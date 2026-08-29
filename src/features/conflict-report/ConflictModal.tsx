import { useDialogSync } from '../../hooks/useDialogSync';
import { AlertTriangle, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '../../types/scanner';
import { useBulkToggle } from '../../hooks/useBulkModMutations';
import { commands } from '../../lib/bindings';
import { formatAppError } from '../../lib/appError';
import ConflictGroupCard from './ConflictGroupCard';
import ConflictResolutionSummary from './ConflictResolutionSummary';
import {
  buildConflictKey,
  chooseConflictWinner,
  setModDecision,
  summarizeConflictResolution,
  type ConflictDecisions,
} from './conflictResolution';

interface ConflictModalProps {
  open: boolean;
  onClose: () => void;
  conflicts: ConflictInfo[];
  gameId: string;
}

export default function ConflictModal({ open, onClose, conflicts, gameId }: ConflictModalProps) {
  const { t } = useTranslation(['scanner', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const bulkToggle = useBulkToggle();
  const [decisions, setDecisions] = useState<ConflictDecisions>(new Map());
  const [reviewing, setReviewing] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [pathErrors, setPathErrors] = useState<ReadonlyMap<string, string>>(new Map());
  const [actionError, setActionError] = useState<string | null>(null);

  useDialogSync(dialogRef, open);

  useEffect(() => {
    if (open) return;
    setDecisions(new Map());
    setReviewing(false);
    setPathErrors(new Map());
    setActionError(null);
  }, [open]);

  const summary = useMemo(
    () => summarizeConflictResolution(conflicts, decisions),
    [conflicts, decisions],
  );
  const isBusy = isSubmitting || bulkToggle.isPending;

  const handleClose = () => {
    if (isBusy) return;
    setDecisions(new Map());
    setReviewing(false);
    setPathErrors(new Map());
    setActionError(null);
    onClose();
  };

  const handleOpenFolder = async (path: string) => {
    setActionError(null);
    try {
      await commands.openInExplorer(gameId, path);
    } catch (error) {
      setActionError(
        t('scanner:conflict_modal.open_failed', {
          error: formatAppError(error),
        }),
      );
    }
  };

  const handleDisable = async () => {
    if (isBusy || summary.disablePaths.length === 0) return;

    setIsSubmitting(true);
    setActionError(null);
    setPathErrors(new Map());
    try {
      const result = await bulkToggle.mutateAsync({
        gameId,
        paths: summary.disablePaths,
        enable: false,
      });
      const failures = new Map(
        result.failures.map((failure) => [failure.path, formatAppError(failure.error)]),
      );
      setPathErrors(failures);
      setDecisions(new Map(result.failures.map((failure) => [failure.path, 'disable'] as const)));
      setReviewing(false);
    } catch (error) {
      setActionError(
        t('scanner:conflict_modal.submit_failed', {
          error: formatAppError(error),
        }),
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="modal bg-overlay-mask backdrop-blur-sm"
      onClose={handleClose}
    >
      <div className="modal-box w-11/12 max-w-4xl border border-warning/20 bg-base-100 shadow-2xl">
        <button
          className="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
          onClick={handleClose}
          disabled={isBusy}
          aria-label={t('common:actions.close')}
        >
          <X size={18} />
        </button>

        <h3 className="font-bold text-lg text-warning flex items-center gap-2 pb-4 border-b border-base-content/10">
          <AlertTriangle className="fill-warning/20" />
          {t('scanner:conflict_modal.title')}
        </h3>

        <div className="py-4 space-y-4 max-h-[65vh] overflow-y-auto">
          {actionError && <div className="alert alert-error text-sm">{actionError}</div>}
          {conflicts.length === 0 ? (
            <p className="text-success text-center italic">{t('scanner:conflict_modal.empty')}</p>
          ) : reviewing ? (
            <ConflictResolutionSummary
              summary={summary}
              isPending={isBusy}
              onBack={() => setReviewing(false)}
              onConfirm={handleDisable}
            />
          ) : (
            <div className="flex flex-col gap-3">
              <div className="alert alert-warning text-xs shadow-sm">
                <span>{t('scanner:conflict_modal.description')}</span>
              </div>

              {conflicts.map((conflict) => (
                <ConflictGroupCard
                  key={buildConflictKey(conflict)}
                  conflict={conflict}
                  decisions={decisions}
                  pathErrors={pathErrors}
                  disabled={isBusy}
                  onKeep={(path) => {
                    setPathErrors(new Map());
                    setDecisions((current) => chooseConflictWinner(current, conflict, path));
                  }}
                  onDisable={(path) => {
                    setPathErrors(new Map());
                    setDecisions((current) => setModDecision(current, path, 'disable'));
                  }}
                  onOpenFolder={(path) => void handleOpenFolder(path)}
                />
              ))}
            </div>
          )}
        </div>

        {!reviewing && conflicts.length > 0 && (
          <div className="modal-action border-t border-base-content/10 pt-4 flex-wrap">
            <div className="mr-auto text-xs text-base-content/60">
              {t('scanner:conflict_modal.impact', {
                resolved: summary.resolvedCount,
                total: conflicts.length,
                count: summary.disablePaths.length,
              })}
            </div>
            {summary.disablePaths.length > 0 && (
              <button
                className="btn btn-sm btn-ghost"
                onClick={() => {
                  setDecisions(new Map());
                  setPathErrors(new Map());
                }}
                disabled={isBusy}
              >
                {t('scanner:conflict_modal.clear_choices')}
              </button>
            )}
            <button className="btn btn-sm btn-ghost" onClick={handleClose} disabled={isBusy}>
              {t('common:actions.close')}
            </button>
            <button
              className="btn btn-sm btn-warning"
              onClick={() => setReviewing(true)}
              disabled={isBusy || summary.disablePaths.length === 0}
            >
              {t('scanner:conflict_modal.review_changes')}
            </button>
          </div>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={handleClose} disabled={isBusy}>
          {t('common:actions.close')}
        </button>
      </form>
    </dialog>
  );
}
