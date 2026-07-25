/** Pure message formatting for bulk mod-operation toasts. */

import i18n from '../lib/i18n';
import { formatAppError } from '../lib/appError';
import { stripDisabledPrefix } from '../lib/disabledPrefix';

export type BulkSuccessActionKey =
  | 'enabled'
  | 'disabled'
  | 'deleted'
  | 'updated'
  | 'favorited'
  | 'unfavorited'
  | 'pinned'
  | 'unpinned';

export type BulkFailureActionKey = 'toggle' | 'delete' | 'update' | 'favorite' | 'pin' | 'import';

export const BULK_TOAST_PREVIEW_LIMIT = 4;

/**
 * Success toast for a finished bulk batch. `displayNames` is already resolved by
 * the caller (cached folder name when known, path segment otherwise); only the
 * first few are named, the rest collapse into a counter.
 */
export function formatBulkSuccessMessage(
  displayNames: string[],
  actionKey: BulkSuccessActionKey,
): string {
  const count = displayNames.length;
  if (count === 0) return '';

  const action = i18n.t(`grid:bulk_toast.actions.${actionKey}`);
  const names = displayNames.slice(0, BULK_TOAST_PREVIEW_LIMIT).join(', ');
  if (count <= BULK_TOAST_PREVIEW_LIMIT) {
    return i18n.t('grid:bulk_toast.success', {
      action,
      count,
      names,
    });
  }

  return i18n.t('grid:bulk_toast.success_with_more', {
    action,
    count,
    extraCount: count - BULK_TOAST_PREVIEW_LIMIT,
    names,
  });
}

/** Failure toast naming the first failing folder plus a counter for the rest. */
export function formatBulkFailureMessage(
  failures: { path: string; error: unknown }[],
  actionKey: BulkFailureActionKey,
): string {
  if (failures.length === 0) {
    return '';
  }

  const firstFailure = failures[0];
  const firstName = stripDisabledPrefix(
    firstFailure.path.split(/[/\\]/).pop() || firstFailure.path,
  );
  const reason = formatAppError(firstFailure.error);
  const action = i18n.t(`grid:bulk_toast.failure_actions.${actionKey}`);
  if (failures.length === 1) {
    return i18n.t('grid:bulk_toast.failure_one', {
      action,
      name: firstName,
      reason,
    });
  }

  return i18n.t('grid:bulk_toast.failure_other', {
    action,
    extraCount: failures.length - 1,
    name: firstName,
    reason,
  });
}
