import { openWorkspaceFileInUseDialog } from '../features/workspace-runtime/state/workspaceDialogs';
import { extractFileInUsePayload } from '../lib/appError';

/**
 * If the error is a structured FileInUse, open the retry dialog wired to re-run
 * the same mutation and report that the error was handled. Every mutation that
 * touches files on disk shares this branch.
 */
export function openFileInUseRetryDialog<TVariables>(
  error: unknown,
  variables: TVariables,
  retry: (variables: TVariables) => void,
): boolean {
  const payload = extractFileInUsePayload(error);
  if (!payload) {
    return false;
  }

  openWorkspaceFileInUseDialog({
    path: payload.path,
    processes: payload.processes,
    onRetry: () => retry(variables),
  });
  return true;
}
