import { openWorkspaceFileInUseDialog } from '../state/workspaceDialogs';
import {
  extractFileInUsePayload,
  FileLockDiagnosticError,
  formatAppError,
} from '@/shared/lib/appError';
import { canonicalPathKey } from '@/shared/lib/pathKey';
import { useAppStore } from '@/app/store';
import i18next from '@/shared/lib/i18n';
import { commands } from '@/shared/api/tauri/bindings';

const activeRetries = new Set<unknown>();

async function dismissRecoveryContexts(recoveryIds: Iterable<string>): Promise<void> {
  const results = await Promise.allSettled(
    Array.from(recoveryIds, (id) => commands.dismissFileLockRecovery(id)),
  );
  const failed = results.find((result) => result.status === 'rejected');
  if (failed?.status === 'rejected') throw failed.reason;
}

export class FileLockRecoveryCancelled extends Error {
  constructor() {
    super(i18next.t('common:actions.cancel'));
    this.name = 'FileLockRecoveryCancelled';
  }
}

/** Pauses a sequence at one uncommitted step until recovery succeeds or is cancelled. */
export async function runWithFileLockRecovery<TData>(
  gameId: string,
  execute: (recoveryId: string | null) => Promise<TData>,
): Promise<TData> {
  const assertCurrentGame = () => {
    if (useAppStore.getState().activeGameId !== gameId) throw new FileLockRecoveryCancelled();
  };
  try {
    assertCurrentGame();
    return await execute(null);
  } catch (error) {
    const lock = extractFileInUsePayload(error);
    if (!lock) throw error;
    assertCurrentGame();
    const recoveryIds = new Set(lock.recoveryId ? [lock.recoveryId] : []);
    return new Promise<TData>((resolve, reject) => {
      let settled = false;
      openWorkspaceFileInUseDialog({
        ...lock,
        onRetry: async (recoveryId) => {
          if (settled) throw new FileLockRecoveryCancelled();
          try {
            assertCurrentGame();
            const id = recoveryId ?? lock.recoveryId ?? null;
            if (id) recoveryIds.add(id);
            const result = await execute(id);
            if (settled) throw new FileLockRecoveryCancelled();
            settled = true;
            void dismissRecoveryContexts(recoveryIds).catch((dismissError: unknown) => {
              console.error(
                '[FileLock] Could not dismiss completed step recovery:',
                formatAppError(dismissError),
              );
            });
            resolve(result);
            return result;
          } catch (retryError) {
            const nextLock = extractFileInUsePayload(retryError);
            if (nextLock?.recoveryId) recoveryIds.add(nextLock.recoveryId);
            if (!nextLock && !settled) {
              settled = true;
              reject(retryError);
            }
            throw retryError;
          }
        },
        onDismiss: async () => {
          if (!settled) {
            settled = true;
            reject(new FileLockRecoveryCancelled());
          }
          await dismissRecoveryContexts(recoveryIds);
        },
      });
    });
  }
}

export function isFileLockRetryInProgress(variables: unknown): boolean {
  return activeRetries.has(variables);
}

type LockFailure = { path: string; error: unknown };
type BulkRetryResult = { success: string[]; failures: LockFailure[] };
type RecoverableFailure = LockFailure & { recoveryId: string };

function recoverableFailures(failures: LockFailure[]): RecoverableFailure[] {
  return failures.flatMap((failure) => {
    const recoveryId = extractFileInUsePayload(failure.error)?.recoveryId;
    return recoveryId ? [{ ...failure, recoveryId }] : [];
  });
}

export function openBulkFileInUseRetryDialog<TVariables extends { paths: string[] }>(
  result: BulkRetryResult,
  variables: TVariables,
  retry: (variables: TVariables & { recoveryIds: string[] }) => Promise<BulkRetryResult>,
): boolean {
  if (isFileLockRetryInProgress(variables)) return false;
  const remaining = recoverableFailures(result.failures).filter((failure) =>
    variables.paths.some((path) => canonicalPathKey(path) === canonicalPathKey(failure.path)),
  );
  if (remaining.length === 0) return false;
  return openBulkRecoveryDialog(
    remaining[0].error,
    variables,
    remaining.map((failure) => failure.path),
    remaining.map((failure) => failure.recoveryId),
    retry,
  );
}

function openBulkRecoveryDialog<TVariables extends { paths: string[] }>(
  error: unknown,
  variables: TVariables,
  originalPendingPaths: string[],
  originalRecoveryIds: string[],
  retry: (variables: TVariables & { recoveryIds: string[] }) => Promise<BulkRetryResult>,
): boolean {
  let pendingPaths = originalPendingPaths;
  let pendingRecoveryIds = originalRecoveryIds;
  const recoveryIds = new Set(pendingRecoveryIds);
  return openFileInUseRetryDialog(
    error,
    variables,
    async (dialogVariables) => {
      if (pendingPaths.length === 0) throw new Error(i18next.t('common:file_in_use.retry_blocked'));
      if ('recoveryId' in dialogVariables && typeof dialogVariables.recoveryId === 'string') {
        pendingRecoveryIds = [dialogVariables.recoveryId, ...pendingRecoveryIds.slice(1)];
        recoveryIds.add(dialogVariables.recoveryId);
      }
      const refreshed = await Promise.all(
        pendingRecoveryIds.map(async (id) => {
          const info = await commands.getFileLockInfo(id).catch((diagnosticError: unknown) => {
            throw FileLockDiagnosticError.fromIo(diagnosticError) ?? diagnosticError;
          });
          recoveryIds.add(info.recovery_id);
          return info.recovery_id;
        }),
      );
      pendingRecoveryIds = refreshed;
      const retryVariables = {
        ...variables,
        paths: pendingPaths,
        recoveryIds: pendingRecoveryIds,
      };
      activeRetries.add(retryVariables);
      try {
        const next = await retry(retryVariables);
        const remaining = recoverableFailures(next.failures).filter((failure) =>
          pendingPaths.some((path) => canonicalPathKey(path) === canonicalPathKey(failure.path)),
        );
        pendingPaths = remaining.map((failure) => failure.path);
        pendingRecoveryIds = remaining.map((failure) => failure.recoveryId);
        for (const failure of remaining) recoveryIds.add(failure.recoveryId);
        if (remaining.length > 0) throw remaining[0].error;
        if (next.failures.length > 0) throw new Error(formatAppError(next.failures[0].error));
        return next;
      } finally {
        activeRetries.delete(retryVariables);
      }
    },
    async () => {
      await dismissRecoveryContexts(recoveryIds);
    },
  );
}

export function openBulkFileLockErrorDialog<TVariables extends { paths: string[] }>(
  error: unknown,
  variables: TVariables,
  retry: (variables: TVariables & { recoveryIds: string[] }) => Promise<BulkRetryResult>,
): boolean {
  const lock = extractFileInUsePayload(error);
  if (!lock) return false;
  if (isFileLockRetryInProgress(variables)) return true;
  if (lock.recoveryId) {
    // Thrown errors describe a rolled-back transaction; every original target is still pending.
    return openBulkRecoveryDialog(error, variables, variables.paths, [lock.recoveryId], retry);
  }
  openWorkspaceFileInUseDialog({
    path: lock.path,
    processes: lock.processes,
    ...(lock.recoveryId ? { recoveryId: lock.recoveryId } : {}),
  });
  return true;
}

/**
 * If the error is a structured FileInUse, open the retry dialog wired to re-run
 * the same mutation and report that the error was handled. Every mutation that
 * touches files on disk shares this branch.
 */
export function openFileInUseRetryDialog<TVariables>(
  error: unknown,
  variables: TVariables,
  retry: (variables: TVariables) => Promise<unknown>,
  onDismiss?: () => Promise<void>,
): boolean {
  const payload = extractFileInUsePayload(error);
  if (!payload) {
    return false;
  }
  if (activeRetries.has(variables)) return true;

  const gameId = useAppStore.getState().activeGameId;
  if (
    typeof variables === 'object' &&
    variables !== null &&
    'gameId' in variables &&
    variables.gameId !== gameId
  )
    return true;

  openWorkspaceFileInUseDialog({
    path: payload.path,
    processes: payload.processes,
    ...(payload.recoveryId ? { recoveryId: payload.recoveryId } : {}),
    ...(onDismiss ? { onDismiss } : {}),
    onRetry: async (recoveryId) => {
      if (useAppStore.getState().activeGameId !== gameId) {
        throw new Error(i18next.t('common:file_in_use.context_changed'));
      }
      const retryVariables =
        recoveryId && typeof variables === 'object' && variables !== null
          ? { ...variables, recoveryId }
          : variables;
      activeRetries.add(retryVariables);
      try {
        return await retry(retryVariables);
      } finally {
        activeRetries.delete(retryVariables);
      }
    },
  });
  return true;
}
