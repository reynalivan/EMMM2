import { toast } from '../stores/useToastStore';
import i18next from './i18n';

type CommittedMutationResult = {
  sync_warning?: {
    kind?: string;
    message: string;
  } | null;
  syncWarning?: {
    kind?: string;
    message: string;
  } | null;
};

/**
 * The disk mutation is already durable. Surface projection lag as a warning,
 * never as a failed action that encourages users to repeat rename/delete.
 */
export function notifyCommittedMutationSyncWarning(
  result: CommittedMutationResult | null | undefined,
): void {
  if (!result) {
    return;
  }
  const warning = result.sync_warning ?? result.syncWarning;
  if (!warning) {
    return;
  }

  toast.warning(
    i18next.t('common:reconcile.runtime_effects_pending', { error: warning.message }),
    7000,
  );
}
