/**
 * DropConfirmModal — shown when the user drops files onto an object
 * whose match confidence is low (≤ 50%).
 * Offers: Move Anyway | Move to Suggested | Cancel
 * During validation loading: shows spinner + Skip Validation button.
 */

import { useEffect, useRef } from 'react';
import { AlertTriangle, FolderInput, Loader2, SkipForward } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface DropValidation {
  /** Paths being dropped */
  paths: string[];
  /** Target object the user dropped onto */
  targetId: string;
  targetName: string;
  /** Validation state */
  status: 'validating' | 'warning';
  /** Score of the target object (0-100) */
  targetScore?: number;
  /** Best match suggestion (if different from target) */
  suggestedId?: string;
  suggestedName?: string;
  suggestedScore?: number;
}

interface DropConfirmModalProps {
  validation: DropValidation | null;
  onMoveAnyway: () => void;
  onMoveToSuggested: () => void;
  onCancel: () => void;
  onSkipValidation: () => void;
}

export default function DropConfirmModal({
  validation,
  onMoveAnyway,
  onMoveToSuggested,
  onCancel,
  onSkipValidation,
}: DropConfirmModalProps) {
  const { t } = useTranslation(['objects', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (validation && !dialog.open) dialog.showModal();
    else if (!validation && dialog.open) dialog.close();
  }, [validation]);

  if (!validation) return null;

  const isValidating = validation.status === 'validating';
  const hasSuggestion =
    validation.suggestedId &&
    validation.suggestedId !== validation.targetId &&
    (validation.suggestedScore ?? 0) > (validation.targetScore ?? 0);

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onCancel}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-md">
        {isValidating ? (
          /* Loading / Validating state */
          <div className="flex flex-col items-center gap-4 py-4">
            <Loader2 size={32} className="animate-spin text-primary" />
            <div className="text-center">
              <p className="text-sm text-base-content/70">{t('drop_confirm.validating')}</p>
              <p className="text-xs text-base-content/40 mt-1">
                {t('drop_confirm.validating_description', {
                  count: validation.paths.length,
                  targetName: validation.targetName,
                })}
              </p>
            </div>
            <button className="btn btn-sm btn-ghost gap-1.5" onClick={onSkipValidation}>
              <SkipForward size={14} />
              {t('drop_confirm.skip_validation')}
            </button>
          </div>
        ) : (
          /* Warning state — low confidence match */
          <>
            <div className="flex items-start gap-3">
              <div className="p-2 rounded-lg bg-warning/10 text-warning shrink-0">
                <AlertTriangle size={20} />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="font-semibold text-base text-base-content">
                  {t('drop_confirm.title')}
                </h3>
                <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
                  {t('drop_confirm.description', { targetName: validation.targetName })}
                  {validation.targetScore !== undefined && (
                    <span className="text-warning font-medium">
                      {' '}
                      {t('drop_confirm.match_score', { score: validation.targetScore })}
                    </span>
                  )}
                </p>
                {hasSuggestion && (
                  <div className="mt-3 p-2.5 rounded-lg bg-primary/5 border border-primary/20">
                    <p className="text-xs text-base-content/50 mb-1">
                      {t('drop_confirm.suggested_target')}
                    </p>
                    <div className="flex items-center gap-2">
                      <FolderInput size={14} className="text-primary" />
                      <span className="text-sm font-medium text-primary">
                        {validation.suggestedName}
                      </span>
                      <span className="text-xs text-base-content/40">
                        {t('drop_confirm.match_score', {
                          score: validation.suggestedScore ?? 0,
                        })}
                      </span>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="modal-action mt-4 flex-wrap gap-2">
              <button className="btn btn-sm btn-ghost" onClick={onCancel}>
                {t('common:actions.cancel')}
              </button>
              <button className="btn btn-sm btn-warning" onClick={onMoveAnyway}>
                {t('drop_confirm.move_anyway')}
              </button>
              {hasSuggestion && (
                <button className="btn btn-sm btn-primary" onClick={onMoveToSuggested}>
                  {t('drop_confirm.move_to', { targetName: validation.suggestedName })}
                </button>
              )}
            </div>
          </>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
