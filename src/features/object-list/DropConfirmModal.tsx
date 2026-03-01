/**
 * DropConfirmModal — shown when the user drops files onto an object
 * whose match confidence is low (≤ 50%).
 * Offers: Move Anyway | Move to Suggested | Cancel
 * During validation loading: shows spinner + Skip Validation button.
 */

import { useEffect, useRef } from 'react';
import { AlertTriangle, FolderInput, Loader2, SkipForward } from 'lucide-react';

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
              <p className="text-sm text-base-content/70">Checking match confidence...</p>
              <p className="text-xs text-base-content/40 mt-1">
                Validating {validation.paths.length} item(s) against &quot;{validation.targetName}
                &quot;
              </p>
            </div>
            <button className="btn btn-sm btn-ghost gap-1.5" onClick={onSkipValidation}>
              <SkipForward size={14} />
              Skip Validation
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
                <h3 className="font-semibold text-base text-base-content">Low Match Confidence</h3>
                <p className="text-sm text-base-content/60 mt-1 leading-relaxed">
                  The dropped items may not belong to &quot;{validation.targetName}&quot;
                  {validation.targetScore !== undefined && (
                    <span className="text-warning font-medium">
                      {' '}
                      ({validation.targetScore}% match)
                    </span>
                  )}
                </p>
                {hasSuggestion && (
                  <div className="mt-3 p-2.5 rounded-lg bg-primary/5 border border-primary/20">
                    <p className="text-xs text-base-content/50 mb-1">Suggested target:</p>
                    <div className="flex items-center gap-2">
                      <FolderInput size={14} className="text-primary" />
                      <span className="text-sm font-medium text-primary">
                        {validation.suggestedName}
                      </span>
                      <span className="text-xs text-base-content/40">
                        ({validation.suggestedScore}% match)
                      </span>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="modal-action mt-4 flex-wrap gap-2">
              <button className="btn btn-sm btn-ghost" onClick={onCancel}>
                Cancel
              </button>
              <button className="btn btn-sm btn-warning" onClick={onMoveAnyway}>
                Move Anyway
              </button>
              {hasSuggestion && (
                <button className="btn btn-sm btn-primary" onClick={onMoveToSuggested}>
                  Move to {validation.suggestedName}
                </button>
              )}
            </div>
          </>
        )}
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onCancel}>close</button>
      </form>
    </dialog>
  );
}
