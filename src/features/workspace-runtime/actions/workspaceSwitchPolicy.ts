import type { TFunction } from 'i18next';
import type { WorkspaceNode } from '../../../types/workspace';
import { formatWorkspaceReason } from '../workspaceSemantics';

export interface WorkspaceSwitchPolicy {
  canToggle: boolean;
  checked: boolean;
  blocked: boolean;
  label: string;
}

export function isWorkspaceSwitchChecked(node: WorkspaceNode | null | undefined): boolean {
  return node?.switch_state === 'enabled';
}

export function getWorkspaceSwitchNextEnabledState(
  node: WorkspaceNode | null | undefined,
): boolean {
  return !isWorkspaceSwitchChecked(node);
}

export function buildWorkspaceSwitchPolicy(
  t: TFunction,
  node: WorkspaceNode | null | undefined,
): WorkspaceSwitchPolicy {
  if (!node) {
    return {
      canToggle: false,
      checked: false,
      blocked: false,
      label: t('common:status.disabled'),
    };
  }

  const checked = isWorkspaceSwitchChecked(node);
  const blocked = node.switch_state === 'blocked_by_ancestor';
  const reasonLabel = formatWorkspaceReason(t, node.switch_reason);
  const label = reasonLabel ?? t(checked ? 'common:status.enabled' : 'common:status.disabled');

  return {
    canToggle: node.capabilities.can_toggle && !blocked,
    checked,
    blocked,
    label,
  };
}
