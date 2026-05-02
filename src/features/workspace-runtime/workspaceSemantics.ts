import type { TFunction } from 'i18next';
import type { WorkspaceReason, WorkspaceWarning } from '../../types/workspace';

function getReasonText(t: TFunction, reason: WorkspaceReason): string {
  if (reason.code === 'disabled_by_container') {
    return t('common:workspace.reason.disabled_by_container', {
      name: reason.args.container_name ?? '',
    });
  }

  if (reason.code === 'object_folder_disabled') {
    return t('common:workspace.reason.object_folder_disabled');
  }

  return '';
}

export function formatWorkspaceReason(
  t: TFunction,
  reason: WorkspaceReason | null | undefined,
): string | null {
  if (!reason) {
    return null;
  }

  const text = getReasonText(t, reason);
  return text.trim().length > 0 ? text : null;
}

export function formatWorkspaceWarning(
  t: TFunction,
  warning: WorkspaceWarning | null | undefined,
): string | null {
  if (!warning) {
    return null;
  }

  if (warning.code === 'folder_warning') {
    return warning.args.message ?? null;
  }

  if (warning.code === 'inactive_reason') {
    const reasonCode = warning.args.reason_code;
    if (!reasonCode) {
      return null;
    }

    return formatWorkspaceReason(t, {
      code: reasonCode as WorkspaceReason['code'],
      args: warning.args,
    });
  }

  if (warning.code === 'naming_conflict') {
    return t('common:workspace.warning.naming_conflict');
  }

  return null;
}
