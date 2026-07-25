import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ApplyResult } from '../../../types/collection';

interface ApplyCollectionActionsProps {
  result: ApplyResult | null;
  missingPaths: string[] | null;
  isApplying: boolean;
  isReplacing: boolean;
  isPreviewLoading: boolean;
  onClose: () => void;
  onDismissMissing: () => void;
  onConfirm: () => void;
  onUpdateOriginal: () => void;
}

/** Action bar of the apply-collection modal: which buttons show depends on
 *  whether the apply already ran and whether mods were reported missing. */
export function ApplyCollectionActions({
  result,
  missingPaths,
  isApplying,
  isReplacing,
  isPreviewLoading,
  onClose,
  onDismissMissing,
  onConfirm,
  onUpdateOriginal,
}: ApplyCollectionActionsProps) {
  const { t } = useTranslation(['collections', 'common']);

  const closeLabel = result?.partial_apply
    ? t('collections:apply.partial.keep_unsaved', 'Keep Unsaved')
    : result
      ? t('common:actions.close')
      : t('collections:apply.actions.cancel');

  return (
    <div className="p-4 border-t border-base-content/5 bg-base-300 shrink-0 flex justify-end gap-2">
      <button className="btn btn-ghost" onClick={onClose} disabled={isApplying || isReplacing}>
        {closeLabel}
      </button>

      {missingPaths && !result && (
        <button className="btn btn-ghost" onClick={onDismissMissing} disabled={isApplying}>
          {t('collections:apply.missing.back', 'Back')}
        </button>
      )}

      {!result && (
        <button
          data-testid="modal-apply-btn"
          className="btn btn-primary min-w-30"
          onClick={onConfirm}
          disabled={isApplying || isPreviewLoading}
        >
          {isApplying ? (
            <Loader2 size={16} className="animate-spin" />
          ) : missingPaths ? (
            t('collections:apply.missing.confirm', 'Skip & Apply')
          ) : (
            t('collections:apply.actions.confirm')
          )}
        </button>
      )}

      {result?.partial_apply && (
        <button className="btn btn-primary" onClick={onUpdateOriginal} disabled={isReplacing}>
          {isReplacing ? (
            <Loader2 size={16} className="animate-spin" />
          ) : (
            t('collections:apply.partial.update_original', 'Update Original Collection')
          )}
        </button>
      )}
    </div>
  );
}
