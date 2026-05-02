import type { WorkspaceNode } from '../../../types/workspace';
import type { WorkspaceSwitchPolicy } from '../actions/workspaceSwitchPolicy';

interface WorkspaceSwitchControlProps {
  node: WorkspaceNode | null | undefined;
  policy: WorkspaceSwitchPolicy;
  isPending: boolean;
  size: 'xs' | 'sm';
  ariaLabel: string;
  onToggle: (node: WorkspaceNode) => void;
}

export function WorkspaceSwitchControl({
  node,
  policy,
  isPending,
  size,
  ariaLabel,
  onToggle,
}: WorkspaceSwitchControlProps) {
  return (
    <input
      type="checkbox"
      aria-label={ariaLabel}
      className={`toggle border-base-content/10 bg-base-300 checked:border-primary checked:bg-primary transition-all duration-200 ${
        size === 'xs' ? 'toggle-xs' : 'toggle-sm'
      }`}
      checked={policy.checked}
      disabled={!node || !policy.canToggle || isPending}
      onChange={() => {
        if (!node) {
          return;
        }

        onToggle(node);
      }}
    />
  );
}
