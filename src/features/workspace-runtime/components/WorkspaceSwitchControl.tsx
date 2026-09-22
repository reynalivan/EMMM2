import type { WorkspaceNode } from '@/entities/workspace';
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
  return (
    <span className="relative inline-flex items-center">
      <input
        type="checkbox"
        aria-label={ariaLabel}
        aria-busy={isBusy || undefined}
        className={`toggle workspace-interactive border-base-content/10 bg-base-300 checked:border-primary checked:bg-primary focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2 ${
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
      {isBusy && (
        <span
          role="status"
          aria-label={`${ariaLabel} pending`}
          className={`pointer-events-none absolute inset-0 m-auto loading loading-spinner text-primary ${
            size === 'xs' ? 'loading-xs' : 'loading-sm'
          }`}
        />
      )}
    </span>
  );
}
