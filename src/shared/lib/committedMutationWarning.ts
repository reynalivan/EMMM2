import { toast } from '@/shared/ui/toast';
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

const WARNING_DEDUPE_MS = 30_000;
const recentlyShownWarnings = new Map<string, number>();

function manualReloadToastMessage(message: string): string {
  const binding = /press (.+?) to reload its configuration/i.exec(message)?.[1] ?? 'reload_config';
  return i18next.t('common:reconcile.manual_reload_required', { binding });
}

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

  const key = `${warning.kind ?? 'unknown'}:${warning.message}`;
  const now = Date.now();
  const lastShownAt = recentlyShownWarnings.get(key);
  if (lastShownAt !== undefined && now - lastShownAt < WARNING_DEDUPE_MS) {
    return;
  }
  if (recentlyShownWarnings.size >= 64 && !recentlyShownWarnings.has(key)) {
    const oldestKey = recentlyShownWarnings.keys().next().value;
    if (oldestKey) {
      recentlyShownWarnings.delete(oldestKey);
    }
  }
  recentlyShownWarnings.set(key, now);

  toast.warning(
    warning.kind === 'ManualReloadRequired'
      ? manualReloadToastMessage(warning.message)
      : warning.kind === 'WatcherUnavailable'
        ? warning.message
        : i18next.t(
            warning.kind === 'CleanupPending'
              ? 'common:reconcile.cleanup_pending'
              : 'common:reconcile.runtime_effects_pending',
          ),
    7000,
  );
}
