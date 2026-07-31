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

/** "a, b, c, d" — the named head every bulk toast shows before the counter. */
function previewNames(names: string[]): string {
  return names.slice(0, BULK_TOAST_PREVIEW_LIMIT).join(', ');
}

/**
 * Folder name a user recognises, read straight off the path.
 *
 * ponytail: deliberately NOT `pathBasename` — a trailing-separator path keeps
 * its whole value as the label here, which bulkToastMessages.test.ts pins.
 */
function toDisplayName(path: string): string {
  const segments = path.split(/[/\\]/);
  return stripDisabledPrefix(segments[segments.length - 1] || path);
}

/**
 * "a, b, c, d <more(n)>" — long selections are truncated so a bulk toast stays
 * readable. `more` renders the tail because callers word it differently.
 */
export function truncateNameList(names: string[], more: (extra: number) => string): string {
  if (names.length <= BULK_TOAST_PREVIEW_LIMIT) {
    return names.join(', ');
  }

  return `${previewNames(names)} ${more(names.length - BULK_TOAST_PREVIEW_LIMIT)}`;
}

/**
 * Success toast for a finished bulk batch: only the first few paths are named,
 * the rest collapse into a counter.
 */
export function formatBulkSuccessMessage(
  paths: string[],
  actionKey: BulkSuccessActionKey,
): string {
  const count = paths.length;
  if (count === 0) return '';

  const action = i18n.t(`grid:bulk_toast.actions.${actionKey}`);
  const names = previewNames(paths.map(toDisplayName));
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
  const firstName = toDisplayName(firstFailure.path);
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
