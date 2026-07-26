import type { WorkspaceNode } from '../../../types/workspace';
import type { WorkspaceSwitchPolicy } from '../actions/workspaceSwitchPolicy';

interface WorkspaceSwitchControlProps {
  node: WorkspaceNode | null | undefined;
  policy: WorkspaceSwitchPolicy;
  /** Disables the control (covers global locks and policy blocks, not just switches). */
  isPending: boolean;
  /** A switch for THIS node is in flight — show a spinner while the backend round-trip runs. */
  isBusy?: boolean;
  size: 'xs' | 'sm';
  ariaLabel: string;
  onToggle: (node: WorkspaceNode) => void;
}

export function WorkspaceSwitchControl({
  node,
  policy,
  isPending,
  isBusy = false,
  size,
  ariaLabel,
  onToggle,
}: WorkspaceSwitchControlProps) {
  if (isBusy) {
    return (
      <span
        role="status"
        aria-label={ariaLabel}
        className={`loading loading-spinner text-primary ${size === 'xs' ? 'loading-xs' : 'loading-sm'}`}
      />
    );
  }

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
