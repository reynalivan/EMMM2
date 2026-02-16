/**
 * Confirmation modal for bulk duplicate resolution.
 * Implements focus trap and escape-to-close patterns.
 * Covers: TC-9.5-03 (User confirmation before destructive operations)
 */

import { useRef, useEffect } from 'react';
import { AlertTriangle, Loader2, Trash2, Shield, XCircle } from 'lucide-react';
import type { DupScanGroup, ResolutionAction } from '../../../types/dedup';

interface Props {
  isOpen: boolean;
  selections: Map<string, ResolutionAction>;
  groups: DupScanGroup[];
  onConfirm: () => void;
  onCancel: () => void;
  isPending?: boolean;
}

/**
 * Get summary counts by action type from selections.
 */
const getActionSummary = (
  selections: Map<string, ResolutionAction>,
): { keepA: number; keepB: number; ignore: number } => {
  const summary = { keepA: 0, keepB: 0, ignore: 0 };

  selections.forEach((action) => {
    if (action === 'KeepA') summary.keepA++;
    else if (action === 'KeepB') summary.keepB++;
    else if (action === 'Ignore') summary.ignore++;
  });

  return summary;
};

export default function ResolutionModal({
  isOpen,
  selections,
  groups,
  onConfirm,
  onCancel,
  isPending = false,
}: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Sync dialog state with isOpen prop
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  }, [isOpen]);

  // Handle escape key (DaisyUI modal supports this natively)
  const handleDialogClose = () => {
    if (!isPending) {
      onCancel();
    }
  };

  const summary = getActionSummary(selections);
  const totalDeletions = summary.keepA + summary.keepB;

  return (
    <dialog
      ref={dialogRef}
      className="modal modal-bottom sm:modal-middle"
      onCancel={handleDialogClose}
      aria-modal="true"
      aria-labelledby="modal-title"
      aria-describedby="modal-description"
    >
      <div className="modal-box max-w-2xl">
        {/* Header */}
        <div className="flex items-start gap-3 mb-4">
          <AlertTriangle className="w-6 h-6 text-warning flex-shrink-0" />
          <div>
            <h3 id="modal-title" className="font-bold text-lg">
              Confirm Resolution
            </h3>
            <p id="modal-description" className="text-sm text-base-content/70">
              Review the actions below before applying. Deleted mods will be moved to trash.
            </p>
          </div>
        </div>

        {/* Action Summary */}
        <div className="stats shadow w-full mb-4">
          <div className="stat place-items-center">
            <div className="stat-title flex items-center gap-1">
              <Trash2 className="w-4 h-4 text-error" />
              Deletions
            </div>
            <div className="stat-value text-error">{totalDeletions}</div>
            <div className="stat-desc">Moved to trash</div>
          </div>

          <div className="stat place-items-center">
            <div className="stat-title flex items-center gap-1">
              <Shield className="w-4 h-4 text-warning" />
              Ignored
            </div>
            <div className="stat-value text-warning">{summary.ignore}</div>
            <div className="stat-desc">Whitelisted pairs</div>
          </div>

          <div className="stat place-items-center">
            <div className="stat-title">Total</div>
            <div className="stat-value">{selections.size}</div>
            <div className="stat-desc">Actions</div>
          </div>
        </div>

        {/* Detailed Breakdown */}
        <div className="max-h-64 overflow-y-auto bg-base-200 rounded-lg p-3 mb-4">
          <ul className="space-y-2" role="list" aria-label="Resolution action breakdown">
            {Array.from(selections.entries()).map(([groupId, action]) => {
              const group = groups.find((g) => g.groupId === groupId);
              if (!group || group.members.length !== 2) return null;

              const [memberA, memberB] = group.members;

              return (
                <li key={groupId} className="flex items-start gap-2 text-sm">
                  {action === 'KeepA' && (
                    <>
                      <XCircle className="w-4 h-4 text-error flex-shrink-0 mt-0.5" />
                      <div className="flex-1">
                        <span className="font-medium">Delete:</span>{' '}
                        <span className="text-base-content/70">{memberB.displayName}</span>
                        <br />
                        <span className="text-xs text-success">Keep: {memberA.displayName}</span>
                      </div>
                    </>
                  )}

                  {action === 'KeepB' && (
                    <>
                      <XCircle className="w-4 h-4 text-error flex-shrink-0 mt-0.5" />
                      <div className="flex-1">
                        <span className="font-medium">Delete:</span>{' '}
                        <span className="text-base-content/70">{memberA.displayName}</span>
                        <br />
                        <span className="text-xs text-success">Keep: {memberB.displayName}</span>
                      </div>
                    </>
                  )}

                  {action === 'Ignore' && (
                    <>
                      <Shield className="w-4 h-4 text-warning flex-shrink-0 mt-0.5" />
                      <div className="flex-1">
                        <span className="font-medium">Ignore:</span>{' '}
                        <span className="text-base-content/70">
                          {memberA.displayName} ↔ {memberB.displayName}
                        </span>
                      </div>
                    </>
                  )}
                </li>
              );
            })}
          </ul>
        </div>

        {/* Progress Indicator (shown when pending) */}
        {isPending && (
          <div className="alert alert-info mb-4">
            <Loader2 className="w-5 h-5 animate-spin" />
            <span>Resolving duplicates... This may take a moment.</span>
          </div>
        )}

        {/* Actions */}
        <div className="modal-action">
          <button
            className="btn btn-ghost"
            onClick={onCancel}
            disabled={isPending}
            aria-label="Cancel resolution"
          >
            Cancel
          </button>
          <button
            className="btn btn-primary"
            onClick={onConfirm}
            disabled={isPending}
            aria-label={`Confirm and apply ${selections.size} resolution actions`}
          >
            {isPending ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Applying...
              </>
            ) : (
              `Confirm (${selections.size})`
            )}
          </button>
        </div>
      </div>

      {/* Backdrop (DaisyUI modal-backdrop) */}
      <form method="dialog" className="modal-backdrop">
        <button type="button" onClick={handleDialogClose} disabled={isPending}>
          close
        </button>
      </form>
    </dialog>
  );
}
