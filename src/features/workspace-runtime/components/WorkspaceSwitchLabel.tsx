import type { WorkspaceNode } from '../../../types/workspace';
import type { WorkspaceSwitchPolicy } from '../actions/workspaceSwitchPolicy';

interface WorkspaceSwitchLabelProps {
  node: WorkspaceNode | null | undefined;
  policy: WorkspaceSwitchPolicy;
  className?: string;
}

export function WorkspaceSwitchLabel({ node, policy, className }: WorkspaceSwitchLabelProps) {
  if (!node) {
    return null;
  }

  return <span className={className}>{policy.label}</span>;
}
