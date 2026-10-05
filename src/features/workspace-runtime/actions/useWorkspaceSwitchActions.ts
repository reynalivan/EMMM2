import { useCallback, useEffect, useSyncExternalStore } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';
import type {
  WorkspaceExplorerNode,
  WorkspaceNode,
  WorkspaceObjectNode,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '@/entities/workspace';
import type { ModFolder } from '@/entities/game-object';
import { canonicalPathKey, identityPathKey } from '@/shared/lib/pathKey';
import { formatBulkSuccessMessage } from '@/shared/lib/hooks/bulkToastMessages';
import {
  dispatchWorkspaceRuntimeEvent,
  getWorkspaceRuntimeState,
} from '../state/workspaceStoreBridge';
import {
  admitWorkspaceIntentOverride,
  applyWorkspaceSwitchEffects,
  buildNodePendingKey,
  executeWorkspaceSwitch,
  isWorkspaceGameCurrent,
  isWorkspaceObjectNode,
  nextWorkspaceIntentRevision,
  subscribeWorkspaceRootEpoch,
  primeWorkspaceRootEpoch,
  invalidateWorkspaceRootEpoch,
  type WorkspaceSwitchSurface,
  type WorkspaceSwitchEffectsOptions,
} from './workspaceSwitchOps';

export type { WorkspaceSwitchSurface } from './workspaceSwitchOps';

function dialogFolder(
  path: string,
  name?: string,
  id: string | null = null,
  physicalIdentity?: string | null,
): Pick<ModFolder, 'id' | 'path' | 'name' | 'filesystem_identity'> {
  const segments = path.replace(/\\/g, '/').split('/').filter(Boolean);
  const fallbackName = segments[segments.length - 1] ?? path;
  return {
    id,
    path,
    name: name ?? fallbackName,
    ...(physicalIdentity ? { filesystem_identity: physicalIdentity } : {}),
  };
}

const FOLDER_SUCCESS_TOAST_SETTLE_MS = 500;

interface PendingFolderSuccessToast {
  gameId: string;
  changes: Map<string, { path: string; enabled: boolean }>;
  timer: ReturnType<typeof setTimeout>;
}

let pendingFolderSuccessToast: PendingFolderSuccessToast | null = null;

function showFolderDiskCommitToast(
  gameId: string,
  result: WorkspaceSwitchResult,
  enabled: boolean,
  notify = true,
) {
  if (
    !notify ||
    result.status !== 'applied' ||
    typeof result.disk_revision !== 'number' ||
    !result.primary_path
  ) {
    return;
  }

  if (pendingFolderSuccessToast && pendingFolderSuccessToast.gameId !== gameId) {
    clearTimeout(pendingFolderSuccessToast.timer);
    pendingFolderSuccessToast = null;
  }
  const changes =
    pendingFolderSuccessToast?.changes ?? new Map<string, { path: string; enabled: boolean }>();
  changes.set(identityPathKey(result.primary_path) ?? result.primary_path, {
    path: result.primary_path,
    enabled,
  });
  if (pendingFolderSuccessToast) {
    clearTimeout(pendingFolderSuccessToast.timer);
  }
  const timer = setTimeout(() => {
    if (pendingFolderSuccessToast?.timer !== timer) return;
    pendingFolderSuccessToast = null;
    if (!isWorkspaceGameCurrent(gameId)) return;

    const committed = [...changes.values()];
    const action = committed.every((change) => change.enabled)
      ? 'enabled'
      : committed.every((change) => !change.enabled)
        ? 'disabled'
        : 'updated';
    toast.success(
      formatBulkSuccessMessage(
        committed.map((change) => change.path),
        action,
      ),
    );
  }, FOLDER_SUCCESS_TOAST_SETTLE_MS);
  pendingFolderSuccessToast = { gameId, changes, timer };
}

interface WorkspaceNodeSwitchOptions extends WorkspaceSwitchEffectsOptions {
  /** Skips transient feedback when a newer toggle intent has superseded this result. */
  shouldNotifyResult?: () => boolean;
  intentRevision?: number;
  onSourceEpochResolved?: (epoch: string) => void;
}

interface WorkspaceNodeSwitchOutcome {
  path: string;
  settled: Promise<void>;
  resumeSync: () => Promise<void>;
  diskRevision: number | null;
  sourceEpoch?: string;
}

interface LatestNodeToggleIntent {
  desiredEnabled: boolean;
  pendingRevision: number;
  node: WorkspaceNode;
  surface: WorkspaceSwitchSurface;
  options?: WorkspaceSwitchEffectsOptions;
  execute: (
    node: WorkspaceNode,
    desiredEnabled: boolean,
    surface: WorkspaceSwitchSurface,
    options?: WorkspaceNodeSwitchOptions,
  ) => Promise<WorkspaceNodeSwitchOutcome | null>;
  completion: Promise<string | null>;
  resolve: (path: string | null) => void;
  reject: (error: unknown) => void;
}

interface LatestFolderPathIntent {
  path: string;
  expectedIdentity?: string;
  desiredEnabled: boolean;
  pendingRevision: number;
  completion: Promise<string | null>;
  resolve: (path: string | null) => void;
  reject: (error: unknown) => void;
}

interface SwitchRecord {
  physicalIdentity?: string;
  desiredEnabled?: boolean;
  revision?: number;
  pending: boolean;
  node?: WorkspaceNode;
  nodeIntent?: LatestNodeToggleIntent;
  pathIntent?: LatestFolderPathIntent;
  aliases: Set<string>;
  diskObservation?: {
    path: string;
    enabled: boolean;
    revision: number;
    sourceEpoch?: string;
  };
  syncError?: unknown;
  sourceEpochHint?: string;
  resumeSync?: () => void;
  syncRefreshPaused?: boolean;
}

const switchRecords = new Map<string, SwitchRecord>();
const folderAliases = new Map<string, Map<string, string>>();
const rootsByGame = new Map<string, string>();

function removeRecordAliases(key: string, record: SwitchRecord): void {
  if (record.aliases.size === 0) return;
  const gameId = key.slice(0, key.indexOf(':folder:'));
  for (const path of record.aliases) {
    const aliasKey = scopedPendingKey(gameId, path);
    const identities = folderAliases.get(aliasKey);
    const identity = record.physicalIdentity ?? '';
    if (identities?.get(identity) === key) identities.delete(identity);
    if (identities?.size === 0) folderAliases.delete(aliasKey);
  }
  record.aliases.clear();
}

function addRecordAlias(gameId: string, key: string, record: SwitchRecord, path: string): void {
  const canonical = canonicalPathKey(path) ?? path;
  const aliasKey = scopedPendingKey(gameId, canonical);
  let identities = folderAliases.get(aliasKey);
  if (!identities) {
    identities = new Map();
    folderAliases.set(aliasKey, identities);
  }
  identities.set(record.physicalIdentity ?? '', key);
  record.aliases.add(canonical);
}

function deleteSwitchRecord(key: string): void {
  const record = switchRecords.get(key);
  if (record) removeRecordAliases(key, record);
  switchRecords.delete(key);
}

function discardGameSwitchRecords(gameId: string, preserveEpoch?: string): void {
  for (const [key, record] of switchRecords) {
    if (
      key.startsWith(`${gameId}:`) &&
      (preserveEpoch === undefined ||
        (record.diskObservation?.sourceEpoch !== preserveEpoch &&
          record.sourceEpochHint !== preserveEpoch))
    ) {
      deleteSwitchRecord(key);
    }
  }
  publishPendingSnapshot();
}

subscribeWorkspaceRootEpoch((gameId, epoch) => discardGameSwitchRecords(gameId, epoch));

function switchRecord(key: string): SwitchRecord {
  let record = switchRecords.get(key);
  if (!record) {
    record = { pending: false, aliases: new Set() };
    switchRecords.set(key, record);
  }
  return record;
}
const subscribers = new Set<() => void>();
let pendingSnapshotVersion = 0;

function publishPendingSnapshot(): void {
  pendingSnapshotVersion += 1;
  subscribers.forEach((subscriber) => subscriber());
}

function subscribePendingSnapshot(subscriber: () => void): () => void {
  subscribers.add(subscriber);
  return () => subscribers.delete(subscriber);
}

function getPendingSnapshotVersion(): number {
  return pendingSnapshotVersion;
}

function scopedPendingKey(gameId: string, key: string): string {
  return `${gameId}:${key}`;
}

function folderPathPendingKey(gameId: string, path: string, physicalIdentity?: string): string {
  if (physicalIdentity) return scopedPendingKey(gameId, `folder:physical:${physicalIdentity}`);
  const canonical = canonicalPathKey(path) ?? path;
  const identities = folderAliases.get(scopedPendingKey(gameId, canonical));
  const indexedKey = identities?.get('') ?? identities?.values().next().value;
  if (indexedKey) return indexedKey;
  const key = scopedPendingKey(gameId, `folder:${canonical}`);
  return switchRecords.has(key) ? `${key}:location:${nextWorkspaceIntentRevision()}` : key;
}

function nodePendingKey(gameId: string, node: WorkspaceNode): string {
  const ownKey = scopedPendingKey(gameId, buildNodePendingKey(node));
  if (isWorkspaceObjectNode(node)) {
    return ownKey;
  }
  const pathKey = folderPathPendingKey(gameId, node.path, node.filesystem_identity ?? undefined);
  if (!node.id) return pathKey;
  const pathRecord = switchRecords.get(pathKey);
  if (node.id && pathRecord?.node?.id && node.id !== pathRecord.node.id) return ownKey;
  if (
    switchRecords.get(ownKey)?.nodeIntent &&
    (switchRecords.get(ownKey)?.revision ?? 0) >= (switchRecords.get(pathKey)?.revision ?? 0)
  ) {
    return ownKey;
  }
  return pathRecord?.desiredEnabled !== undefined ? pathKey : ownKey;
}

function markPending(key: string, pending: boolean): void {
  const record = switchRecord(key);
  record.pending = pending;
  if (!pending && record.revision === undefined && !record.nodeIntent && !record.pathIntent) {
    deleteSwitchRecord(key);
  }
  publishPendingSnapshot();
}

function setPendingDesired(key: string, desiredEnabled: boolean): number {
  const revision = nextWorkspaceIntentRevision();
  Object.assign(switchRecord(key), {
    revision,
    desiredEnabled,
    syncError: undefined,
    resumeSync: undefined,
    syncRefreshPaused: undefined,
  });
  publishPendingSnapshot();
  return revision;
}

function clearPendingDesired(key: string, expectedRevision: number): void {
  const record = switchRecords.get(key);
  if (record?.revision !== expectedRevision) {
    return;
  }
  record.revision = undefined;
  record.desiredEnabled = undefined;
  record.syncError = undefined;
  record.resumeSync = undefined;
  record.syncRefreshPaused = undefined;
  if (!record.pending && !record.nodeIntent && !record.pathIntent) deleteSwitchRecord(key);
  publishPendingSnapshot();
}

function retainSwitchRefresh(
  gameId: string,
  key: string,
  record: SwitchRecord,
  intentRevision: number,
  diskRevision: number | null,
  sourceEpoch: string | undefined,
  settled: Promise<void>,
  resume: () => Promise<void>,
): void {
  const isCurrentReceipt = () =>
    isWorkspaceGameCurrent(gameId) &&
    switchRecords.get(key) === record &&
    record.revision === intentRevision &&
    (diskRevision === null ||
      (record.diskObservation?.revision === diskRevision &&
        record.diskObservation.sourceEpoch === sourceEpoch));
  let inFlight: Promise<void> | undefined;
  const observe = (completion: Promise<void>) => {
    inFlight = completion;
    void completion.then(
      () => {
        if (inFlight === completion) inFlight = undefined;
        if (!isCurrentReceipt()) return;
        if (record.syncRefreshPaused) {
          record.resumeSync?.();
          return;
        }
        clearPendingDesired(key, intentRevision);
      },
      (error: unknown) => {
        if (inFlight === completion) inFlight = undefined;
        if (!isCurrentReceipt()) return;
        if (record.syncRefreshPaused) {
          record.resumeSync?.();
          return;
        }
        record.syncError = error;
        publishPendingSnapshot();
      },
    );
  };
  if (diskRevision !== null) {
    record.resumeSync = () => {
      if (
        inFlight ||
        !isCurrentReceipt() ||
        record.pending ||
        record.nodeIntent ||
        record.pathIntent
      )
        return;
      if (useAppStore.getState().gameActivationByGame?.[gameId]?.phase !== 'ready') return;
      record.syncRefreshPaused = false;
      observe(resume());
    };
  }
  observe(settled);
}

export function setObjectBulkPendingDesired(
  gameId: string,
  objectIds: Iterable<string>,
  desiredEnabled: boolean,
  revision: number,
): void {
  let changed = false;
  for (const objectId of objectIds) {
    const key = scopedPendingKey(gameId, `object:${objectId}`);
    Object.assign(switchRecord(key), { revision, desiredEnabled });
    changed = true;
  }
  if (changed) publishPendingSnapshot();
}

export function clearObjectBulkPendingDesired(
  gameId: string,
  objectIds: Iterable<string>,
  revision: number,
): void {
  let changed = false;
  for (const objectId of objectIds) {
    const key = scopedPendingKey(gameId, `object:${objectId}`);
    if (switchRecords.get(key)?.revision !== revision) continue;
    clearPendingDesired(key, revision);
    changed = true;
  }
  if (changed) publishPendingSnapshot();
}

export function setFolderBulkPendingDesired(
  gameId: string,
  paths: Iterable<string>,
  desiredEnabled: boolean,
  revision: number,
  expectedIdentities?: [string, string][],
): void {
  const proofs = new Map(
    expectedIdentities?.map(([path, identity]) => [canonicalPathKey(path) ?? path, identity]),
  );
  let changed = false;
  for (const path of paths) {
    const physicalIdentity = proofs.get(canonicalPathKey(path) ?? path);
    const key = folderPathPendingKey(gameId, path, physicalIdentity);
    const record = switchRecord(key);
    record.physicalIdentity = physicalIdentity ?? record.physicalIdentity;
    Object.assign(record, { revision, desiredEnabled });
    addRecordAlias(gameId, key, record, path);
    changed = true;
  }
  if (changed) publishPendingSnapshot();
}

export function clearFolderBulkPendingDesired(
  gameId: string,
  paths: Iterable<string>,
  revision: number,
): void {
  let changed = false;
  for (const path of paths) {
    const canonical = canonicalPathKey(path) ?? path;
    const identities = folderAliases.get(scopedPendingKey(gameId, canonical));
    if (!identities) continue;
    for (const key of [...identities.values()]) {
      if (switchRecords.get(key)?.revision !== revision) continue;
      clearPendingDesired(key, revision);
      changed = true;
    }
  }
  if (changed) publishPendingSnapshot();
}

function withLatestFolderPath(node: WorkspaceNode, latestNode: WorkspaceNode | undefined) {
  if (
    !isWorkspaceObjectNode(node) &&
    latestNode &&
    !isWorkspaceObjectNode(latestNode) &&
    node.filesystem_identity !== latestNode.filesystem_identity
  )
    return node;
  if (isWorkspaceObjectNode(node) || !latestNode || isWorkspaceObjectNode(latestNode)) {
    return node;
  }
  return { ...node, path: latestNode.path };
}

export function useWorkspaceSwitchActions() {
  const { t } = useTranslation(['common', 'objects']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const activationBlocksMutations = useAppStore((state) => {
    if (!activeGame?.id) return false;
    const activation = state.gameActivationByGame?.[activeGame.id];
    return activation?.phase !== 'ready';
  });
  const activationGeneration = useAppStore((state) =>
    activeGame?.id ? state.gameActivationByGame?.[activeGame.id]?.generation : undefined,
  );
  useSyncExternalStore(
    subscribePendingSnapshot,
    getPendingSnapshotVersion,
    getPendingSnapshotVersion,
  );
  useEffect(() => {
    if (!activeGame?.id) return;
    const root = activeGame.mod_path;
    const previousRoot = rootsByGame.get(activeGame.id);
    if (previousRoot !== undefined && previousRoot !== root) {
      discardGameSwitchRecords(activeGame.id);
      invalidateWorkspaceRootEpoch(activeGame.id);
    }
    rootsByGame.set(activeGame.id, root);
    let cancelled = false;
    if (!activationBlocksMutations) {
      const gameId = activeGame.id;
      void primeWorkspaceRootEpoch(gameId)
        .then(() => {
          if (cancelled || !isWorkspaceGameCurrent(gameId)) return;
          for (const [key, record] of switchRecords) {
            if (key.startsWith(`${gameId}:`)) record.resumeSync?.();
          }
        })
        .catch((error: unknown) => {
          console.warn('[WorkspaceSwitch] Could not observe current root epoch:', error);
        });
    }
    return () => {
      cancelled = true;
      // A receipt from this activation must refresh again even if its old promise settles after return.
      for (const [key, record] of switchRecords) {
        if (key.startsWith(`${activeGame.id}:`) && record.resumeSync)
          record.syncRefreshPaused = true;
      }
    };
  }, [activeGame?.id, activeGame?.mod_path, activationBlocksMutations, activationGeneration]);

  const setExplorerNodeEnabled = useCallback(
    async (
      node: WorkspaceExplorerNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ): Promise<WorkspaceNodeSwitchOutcome | null> => {
      if (!activeGame?.id || activationBlocksMutations) {
        return null;
      }
      if (node.filesystem_identity === null) {
        toast.error(t('common:errors.explorer_snapshot_expired'));
        return null;
      }

      const input: WorkspaceSwitchInput = {
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: node.path,
          ...(node.filesystem_identity ? { expected_identity: node.filesystem_identity } : {}),
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        enable_disabled_ancestors: false,
        parent_enable_confirmation: null,
        origin_surface: surface,
      };
      const result = await executeWorkspaceSwitch(input, options?.intentRevision);
      if (result && typeof result.source_epoch === 'string') {
        options?.onSourceEpochResolved?.(result.source_epoch);
      }
      if (!result || !isWorkspaceGameCurrent(input.game_id)) {
        return null;
      }

      if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
        if (options?.shouldNotifyResult?.() === false) {
          return null;
        }
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'folderEnableParent',
            folder: dialogFolder(node.path, node.name, node.id, node.filesystem_identity),
            requirement: result.parent_enable_requirement,
            resumeInput: input,
          },
        });
        return null;
      }

      if (result.status === 'requires_duplicate_resolution') {
        if (options?.shouldNotifyResult?.() === false) {
          return null;
        }
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'modDuplicateWarning',
            folder: dialogFolder(node.path, node.name, node.id, node.filesystem_identity),
            duplicates: result.duplicates,
            requiresResolution: true,
            enableDisabledAncestors: false,
            parentEnableConfirmation: null,
          },
        });
        return null;
      }

      const nextPath = result.primary_path;
      if (!nextPath) {
        return null;
      }

      const shouldNotifyResult = options?.shouldNotifyResult?.() !== false;
      showFolderDiskCommitToast(activeGame.id, result, desiredEnabled, shouldNotifyResult);
      const settled = applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        ...options,
        gameId: activeGame.id,
      });
      if (result.duplicates.length > 0 && shouldNotifyResult) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'modDuplicateWarning',
            folder: dialogFolder(nextPath, node.name, node.id, node.filesystem_identity),
            duplicates: result.duplicates,
            requiresResolution: false,
            enableDisabledAncestors: false,
            parentEnableConfirmation: null,
          },
        });
      }

      return {
        path: nextPath,
        settled,
        resumeSync: () =>
          applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
            ...options,
            gameId: activeGame.id,
            replayPathRewrites: false,
          }),
        diskRevision: result.disk_revision,
        sourceEpoch:
          'source_epoch' in result && typeof result.source_epoch === 'string'
            ? result.source_epoch
            : undefined,
      };
    },
    [activeGame, activationBlocksMutations, queryClient, t],
  );

  const setObjectNodeEnabled = useCallback(
    async (
      node: WorkspaceObjectNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ): Promise<WorkspaceNodeSwitchOutcome | null> => {
      // Explicit object enable/disable stays in Workspace Switch.
      // This path must not rely on Disk Reconcile or mod-toggle semantics.
      if (!activeGame) {
        return null;
      }
      if (activationBlocksMutations) {
        return null;
      }

      const gameId = activeGame.id;
      const result = await executeWorkspaceSwitch(
        {
          game_id: gameId,
          target: {
            kind: 'object_id',
            value: node.id,
          },
          desired_enabled: desiredEnabled,
          resolution: 'normal',
          enable_disabled_ancestors: false,
          parent_enable_confirmation: null,
          origin_surface: surface,
        },
        options?.intentRevision,
      );

      if (!result?.primary_path || !isWorkspaceGameCurrent(gameId)) {
        return null;
      }
      if (typeof result.source_epoch === 'string') {
        options?.onSourceEpochResolved?.(result.source_epoch);
      }

      const nextPath = result.primary_path;
      let settled = Promise.resolve();
      if (result.status !== 'noop') {
        settled = applyWorkspaceSwitchEffects(queryClient, result, 'objectSwitch', {
          ...options,
          gameId,
        });
        if (options?.shouldNotifyResult?.() !== false) {
          toast.success(
            t(desiredEnabled ? 'objects:toasts.enabled_one' : 'objects:toasts.disabled_one', {
              count: 1,
            }),
          );
        }
      }

      return {
        path: nextPath,
        settled,
        resumeSync: () =>
          applyWorkspaceSwitchEffects(queryClient, result, 'objectSwitch', {
            ...options,
            gameId,
            replayPathRewrites: false,
          }),
        diskRevision: result.disk_revision,
        sourceEpoch:
          'source_epoch' in result && typeof result.source_epoch === 'string'
            ? result.source_epoch
            : undefined,
      };
    },
    [activeGame, activationBlocksMutations, queryClient, t],
  );

  const executeNodeEnabled = useCallback(
    async (
      node: WorkspaceNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ): Promise<WorkspaceNodeSwitchOutcome | null> => {
      if (isWorkspaceObjectNode(node)) {
        return setObjectNodeEnabled(node, desiredEnabled, surface, options);
      }

      return setExplorerNodeEnabled(node, desiredEnabled, surface, options);
    },
    [setExplorerNodeEnabled, setObjectNodeEnabled],
  );

  const submitNodeIntent = useCallback(
    (
      node: WorkspaceNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceSwitchEffectsOptions,
    ): Promise<string | null> => {
      if (!activeGame?.id || activationBlocksMutations) {
        return Promise.resolve(null);
      }
      const gameId = activeGame.id;
      const pendingKey = nodePendingKey(gameId, node);
      const record = switchRecord(pendingKey);
      if (!isWorkspaceObjectNode(node))
        record.physicalIdentity = node.filesystem_identity ?? record.physicalIdentity;
      if (!isWorkspaceObjectNode(node)) addRecordAlias(gameId, pendingKey, record, node.path);
      const pathIntent = record.pathIntent;
      if (pathIntent) {
        pathIntent.desiredEnabled = desiredEnabled;
        pathIntent.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        admitWorkspaceIntentOverride(
          activeGame.id,
          [
            {
              kind: 'mod_path',
              value: pathIntent.path,
              ...(pathIntent.expectedIdentity
                ? { expected_identity: pathIntent.expectedIdentity }
                : {}),
            },
          ],
          pathIntent.pendingRevision,
        );
        return pathIntent.completion;
      }
      const existingIntent = record.nodeIntent;
      if (existingIntent) {
        existingIntent.desiredEnabled = desiredEnabled;
        existingIntent.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        existingIntent.node = withLatestFolderPath(node, record.node);
        existingIntent.surface = surface;
        existingIntent.options = options;
        existingIntent.execute = executeNodeEnabled;
        admitWorkspaceIntentOverride(
          activeGame.id,
          [
            isWorkspaceObjectNode(existingIntent.node)
              ? { kind: 'object_id', value: existingIntent.node.id }
              : {
                  kind: 'mod_path',
                  value: existingIntent.node.path,
                  ...(existingIntent.node.filesystem_identity
                    ? { expected_identity: existingIntent.node.filesystem_identity }
                    : {}),
                },
          ],
          existingIntent.pendingRevision,
        );
        return existingIntent.completion;
      }

      let resolve!: (path: string | null) => void;
      let reject!: (error: unknown) => void;
      const completion = new Promise<string | null>((onResolve, onReject) => {
        resolve = onResolve;
        reject = onReject;
      });
      const intent: LatestNodeToggleIntent = {
        desiredEnabled,
        pendingRevision: setPendingDesired(pendingKey, desiredEnabled),
        node: withLatestFolderPath(node, record.node),
        surface,
        options,
        execute: executeNodeEnabled,
        completion,
        resolve,
        reject,
      };
      record.nodeIntent = intent;
      markPending(pendingKey, true);

      const runLatestIntent = async () => {
        let clearDesiredAfterRefresh = false;
        try {
          while (true) {
            const requestedEnabled = intent.desiredEnabled;
            const requestedRevision = intent.pendingRevision;
            const intentNode = intent.node;
            const outcome = await intent.execute(intentNode, requestedEnabled, intent.surface, {
              ...intent.options,
              intentRevision: requestedRevision,
              onSourceEpochResolved: (epoch) => {
                if (switchRecords.get(pendingKey) === record) record.sourceEpochHint = epoch;
              },
              onSyncError: (error: unknown) => {
                if (
                  record.syncRefreshPaused ||
                  !isWorkspaceGameCurrent(gameId) ||
                  switchRecords.get(pendingKey) !== record ||
                  record.revision !== requestedRevision
                )
                  return;
                record.syncError = error;
                publishPendingSnapshot();
              },
              shouldNotifySyncError: () =>
                !record.syncRefreshPaused &&
                isWorkspaceGameCurrent(gameId) &&
                switchRecords.get(pendingKey) === record &&
                record.revision === requestedRevision,
              shouldNotifyResult: () =>
                switchRecords.get(pendingKey) === record &&
                record.nodeIntent === intent &&
                intent.pendingRevision === requestedRevision,
            });
            if (switchRecords.get(pendingKey) !== record) {
              intent.resolve(null);
              return;
            }
            if (outcome && typeof outcome.diskRevision === 'number') {
              record.diskObservation = {
                path: outcome.path,
                enabled: requestedEnabled,
                revision: outcome.diskRevision,
                sourceEpoch: outcome.sourceEpoch,
              };
            }

            if (outcome?.path && !isWorkspaceObjectNode(intentNode)) {
              intent.node = { ...intentNode, path: outcome.path };
              record.node = intent.node;
              removeRecordAliases(pendingKey, record);
              addRecordAlias(gameId, pendingKey, record, outcome.path);
            }
            if (
              intent.desiredEnabled !== requestedEnabled ||
              intent.pendingRevision !== requestedRevision
            ) {
              continue;
            }
            if (!outcome?.path) {
              intent.resolve(null);
              return;
            }

            clearDesiredAfterRefresh = true;
            const completedRevision = intent.pendingRevision;
            retainSwitchRefresh(
              gameId,
              pendingKey,
              record,
              completedRevision,
              outcome.diskRevision,
              outcome.sourceEpoch,
              outcome.settled,
              outcome.resumeSync,
            );
            intent.resolve(outcome.path);
            return;
          }
        } catch (error) {
          intent.reject(error);
        } finally {
          if (record.nodeIntent === intent) {
            record.nodeIntent = undefined;
          }
          if (switchRecords.get(pendingKey) === record) markPending(pendingKey, false);
          if (!clearDesiredAfterRefresh) {
            clearPendingDesired(pendingKey, intent.pendingRevision);
          }
        }
      };
      void runLatestIntent();
      return completion;
    },
    [activeGame?.id, activationBlocksMutations, executeNodeEnabled],
  );

  const setNodeEnabled = submitNodeIntent;

  const toggleNode = useCallback(
    (node: WorkspaceNode, surface: WorkspaceSwitchSurface) => {
      if (!activeGame?.id) {
        return Promise.resolve(null);
      }
      const pendingKey = nodePendingKey(activeGame.id, node);
      const desiredEnabled = !(
        switchRecords.get(pendingKey)?.desiredEnabled ?? node.switch_state === 'enabled'
      );
      return submitNodeIntent(node, desiredEnabled, surface);
    },
    [activeGame, submitNodeIntent],
  );

  const setFolderPathEnabled = useCallback(
    (
      path: string,
      desiredEnabled: boolean,
      expectedIdentity?: string | null,
    ): Promise<string | null> => {
      if (!activeGame?.id || activationBlocksMutations) {
        return Promise.resolve(null);
      }
      if (expectedIdentity === null) {
        toast.error(t('common:errors.explorer_snapshot_expired'));
        return Promise.resolve(null);
      }
      const gameId = activeGame.id;
      const pendingKey = folderPathPendingKey(gameId, path, expectedIdentity ?? undefined);
      const record = switchRecord(pendingKey);
      if (record.nodeIntent && !isWorkspaceObjectNode(record.nodeIntent.node)) {
        return submitNodeIntent(record.nodeIntent.node, desiredEnabled, 'folder_grid');
      }
      record.physicalIdentity = expectedIdentity ?? record.physicalIdentity;
      addRecordAlias(gameId, pendingKey, record, path);
      const existing = record.pathIntent;
      if (existing) {
        existing.desiredEnabled = desiredEnabled;
        existing.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        admitWorkspaceIntentOverride(
          gameId,
          [
            {
              kind: 'mod_path',
              value: existing.path,
              ...(existing.expectedIdentity
                ? { expected_identity: existing.expectedIdentity }
                : {}),
            },
          ],
          existing.pendingRevision,
        );
        return existing.completion;
      }

      let resolve!: (path: string | null) => void;
      let reject!: (error: unknown) => void;
      const completion = new Promise<string | null>((onResolve, onReject) => {
        resolve = onResolve;
        reject = onReject;
      });
      const intent: LatestFolderPathIntent = {
        path,
        expectedIdentity: expectedIdentity ?? undefined,
        desiredEnabled,
        pendingRevision: setPendingDesired(pendingKey, desiredEnabled),
        completion,
        resolve,
        reject,
      };
      record.pathIntent = intent;
      markPending(pendingKey, true);

      const runLatestIntent = async () => {
        let clearDesiredAfterRefresh = false;
        try {
          while (true) {
            const requestedEnabled = intent.desiredEnabled;
            const requestedRevision = intent.pendingRevision;
            const input: WorkspaceSwitchInput = {
              game_id: gameId,
              target: {
                kind: 'mod_path',
                value: intent.path,
                ...(intent.expectedIdentity ? { expected_identity: intent.expectedIdentity } : {}),
              },
              desired_enabled: requestedEnabled,
              resolution: 'normal',
              enable_disabled_ancestors: false,
              parent_enable_confirmation: null,
              origin_surface: 'folder_grid',
            };
            const result = await executeWorkspaceSwitch(input, requestedRevision);
            if (result && typeof result.source_epoch === 'string')
              record.sourceEpochHint = result.source_epoch;
            if (switchRecords.get(pendingKey) !== record) {
              intent.resolve(null);
              return;
            }
            if (!isWorkspaceGameCurrent(gameId)) {
              intent.resolve(null);
              return;
            }
            if (result?.primary_path) {
              if (typeof result.disk_revision === 'number') {
                record.diskObservation = {
                  path: result.primary_path,
                  enabled: requestedEnabled,
                  revision: result.disk_revision,
                  sourceEpoch:
                    'source_epoch' in result && typeof result.source_epoch === 'string'
                      ? result.source_epoch
                      : undefined,
                };
              }
              intent.path = result.primary_path;
              removeRecordAliases(pendingKey, record);
              addRecordAlias(gameId, pendingKey, record, intent.path);
            }
            if (
              intent.desiredEnabled !== requestedEnabled ||
              intent.pendingRevision !== requestedRevision
            ) {
              continue;
            }
            if (!result) {
              intent.resolve(null);
              return;
            }
            if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
              dispatchWorkspaceRuntimeEvent({
                type: 'DIALOG_OPENED',
                dialog: {
                  kind: 'folderEnableParent',
                  folder: dialogFolder(intent.path, undefined, null, intent.expectedIdentity),
                  requirement: result.parent_enable_requirement,
                  resumeInput: input,
                },
              });
              intent.resolve(null);
              return;
            }
            if (result.status === 'requires_duplicate_resolution') {
              dispatchWorkspaceRuntimeEvent({
                type: 'DIALOG_OPENED',
                dialog: {
                  kind: 'modDuplicateWarning',
                  folder: dialogFolder(intent.path, undefined, null, intent.expectedIdentity),
                  duplicates: result.duplicates,
                  requiresResolution: true,
                  enableDisabledAncestors: false,
                  parentEnableConfirmation: null,
                },
              });
              intent.resolve(null);
              return;
            }
            if (!result.primary_path) {
              intent.resolve(null);
              return;
            }

            clearDesiredAfterRefresh = true;
            showFolderDiskCommitToast(gameId, result, requestedEnabled);
            const syncOptions: WorkspaceSwitchEffectsOptions = {
              gameId,
              onSyncError: (error: unknown) => {
                if (
                  record.syncRefreshPaused ||
                  !isWorkspaceGameCurrent(gameId) ||
                  switchRecords.get(pendingKey) !== record ||
                  record.revision !== requestedRevision
                )
                  return;
                record.syncError = error;
                publishPendingSnapshot();
              },
              shouldNotifySyncError: () =>
                !record.syncRefreshPaused &&
                isWorkspaceGameCurrent(gameId) &&
                switchRecords.get(pendingKey) === record &&
                record.revision === requestedRevision,
            };
            const settled = applyWorkspaceSwitchEffects(
              queryClient,
              result,
              'folderSwitch',
              syncOptions,
            );
            const completedRevision = intent.pendingRevision;
            retainSwitchRefresh(
              gameId,
              pendingKey,
              record,
              completedRevision,
              result.disk_revision,
              result.source_epoch ?? undefined,
              settled,
              () =>
                applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
                  ...syncOptions,
                  replayPathRewrites: false,
                }),
            );
            if (result.duplicates.length > 0 && requestedRevision === intent.pendingRevision) {
              dispatchWorkspaceRuntimeEvent({
                type: 'DIALOG_OPENED',
                dialog: {
                  kind: 'modDuplicateWarning',
                  folder: dialogFolder(
                    result.primary_path,
                    undefined,
                    null,
                    intent.expectedIdentity,
                  ),
                  duplicates: result.duplicates,
                  requiresResolution: false,
                  enableDisabledAncestors: false,
                  parentEnableConfirmation: null,
                },
              });
            }
            intent.resolve(result.primary_path);
            return;
          }
        } catch (error) {
          intent.reject(error);
        } finally {
          if (record.pathIntent === intent) record.pathIntent = undefined;
          if (switchRecords.get(pendingKey) === record) markPending(pendingKey, false);
          if (!clearDesiredAfterRefresh) {
            clearPendingDesired(pendingKey, intent.pendingRevision);
          }
        }
      };
      void runLatestIntent();
      return completion;
    },
    [activeGame?.id, activationBlocksMutations, queryClient, submitNodeIntent, t],
  );

  const resolveDuplicateForceEnable = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path' | 'filesystem_identity'> | null,
      enableDisabledAncestors: boolean = false,
      parentEnableConfirmation: string | null = null,
    ) => {
      if (!folder || !activeGame?.id || activationBlocksMutations) {
        return null;
      }

      const input: WorkspaceSwitchInput = {
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: folder.path,
          ...(folder.filesystem_identity ? { expected_identity: folder.filesystem_identity } : {}),
        },
        desired_enabled: true,
        resolution: 'force_enable',
        enable_disabled_ancestors: enableDisabledAncestors,
        parent_enable_confirmation: parentEnableConfirmation,
        origin_surface: 'folder_grid',
      };
      const result = await executeWorkspaceSwitch(input);
      if (!isWorkspaceGameCurrent(input.game_id)) {
        return null;
      }
      if (result?.status === 'requires_parent_enable' && result.parent_enable_requirement) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'folderEnableParent',
            folder: dialogFolder(folder.path, undefined, null, folder.filesystem_identity),
            requirement: result.parent_enable_requirement,
            resumeInput: input,
          },
        });
        return null;
      }
      if (!result?.primary_path) {
        return null;
      }

      showFolderDiskCommitToast(activeGame.id, result, true);
      void applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
      }).catch((error: unknown) => {
        console.warn('[WorkspaceSwitch] Conflict-resolution sync was interrupted:', error);
      });
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, activationBlocksMutations, queryClient],
  );

  const resolveDuplicateEnableOnly = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path' | 'filesystem_identity'> | null,
      enableDisabledAncestors: boolean = false,
      parentEnableConfirmation: string | null = null,
    ) => {
      if (!folder || !activeGame?.id || activationBlocksMutations) {
        return null;
      }

      const input: WorkspaceSwitchInput = {
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: folder.path,
          ...(folder.filesystem_identity ? { expected_identity: folder.filesystem_identity } : {}),
        },
        desired_enabled: true,
        resolution: 'enable_only_this',
        enable_disabled_ancestors: enableDisabledAncestors,
        parent_enable_confirmation: parentEnableConfirmation,
        origin_surface: 'folder_grid',
      };
      const result = await executeWorkspaceSwitch(input);
      if (!result) {
        return null;
      }
      if (!isWorkspaceGameCurrent(input.game_id)) {
        return null;
      }
      if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'folderEnableParent',
            folder: dialogFolder(folder.path, undefined, null, folder.filesystem_identity),
            requirement: result.parent_enable_requirement,
            resumeInput: input,
          },
        });
        return null;
      }

      showFolderDiskCommitToast(activeGame.id, result, true);
      void applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
      }).catch((error: unknown) => {
        console.warn('[WorkspaceSwitch] Conflict-resolution sync was interrupted:', error);
      });
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, activationBlocksMutations, queryClient],
  );

  const resolveParentEnable = useCallback(async () => {
    const dialogState = getWorkspaceRuntimeState().dialogState;
    if (dialogState.kind !== 'folderEnableParent' || activationBlocksMutations) {
      return null;
    }

    const confirmedInput: WorkspaceSwitchInput = {
      ...dialogState.resumeInput,
      enable_disabled_ancestors: true,
      parent_enable_confirmation: dialogState.requirement.confirmation_token,
    };
    const result = await executeWorkspaceSwitch(confirmedInput);
    if (!result || !isWorkspaceGameCurrent(confirmedInput.game_id)) {
      return null;
    }

    if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
      dispatchWorkspaceRuntimeEvent({
        type: 'DIALOG_OPENED',
        dialog: {
          kind: 'folderEnableParent',
          folder: dialogState.folder,
          requirement: result.parent_enable_requirement,
          resumeInput: confirmedInput,
        },
      });
      return null;
    }

    if (result.status === 'requires_duplicate_resolution') {
      dispatchWorkspaceRuntimeEvent({
        type: 'DIALOG_OPENED',
        dialog: {
          kind: 'modDuplicateWarning',
          folder: dialogState.folder,
          duplicates: result.duplicates,
          requiresResolution: true,
          enableDisabledAncestors: true,
          parentEnableConfirmation: dialogState.requirement.confirmation_token,
        },
      });
      return null;
    }
    if (!result.primary_path || !activeGame?.id) {
      return null;
    }

    showFolderDiskCommitToast(activeGame.id, result, true);
    void applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
      gameId: activeGame.id,
    }).catch((error: unknown) => {
      console.warn('[WorkspaceSwitch] Parent-enable sync was interrupted:', error);
    });
    dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'folderEnableParent' });
    if (result.duplicates.length > 0) {
      dispatchWorkspaceRuntimeEvent({
        type: 'DIALOG_OPENED',
        dialog: {
          kind: 'modDuplicateWarning',
          folder: dialogFolder(
            result.primary_path,
            dialogState.folder.name,
            dialogState.folder.id,
            dialogState.folder.filesystem_identity,
          ),
          duplicates: result.duplicates,
          requiresResolution: false,
          enableDisabledAncestors: true,
          parentEnableConfirmation: dialogState.requirement.confirmation_token,
        },
      });
    }
    return result.primary_path;
  }, [activeGame, activationBlocksMutations, queryClient]);

  const isPending = activationBlocksMutations;

  const isNodePending = (node: WorkspaceNode | null | undefined) => {
    if (!node) {
      return false;
    }
    return (
      activationBlocksMutations ||
      (activeGame?.id !== undefined &&
        switchRecords.get(nodePendingKey(activeGame.id, node))?.pending === true)
    );
  };

  const getPendingDesiredEnabled = (node: WorkspaceNode | null | undefined) => {
    if (!node || !activeGame?.id) {
      return undefined;
    }
    return switchRecords.get(nodePendingKey(activeGame.id, node))?.desiredEnabled;
  };

  const getNodeSyncError = (node: WorkspaceNode | null | undefined): unknown => {
    if (!node || !activeGame?.id) return undefined;
    return switchRecords.get(nodePendingKey(activeGame.id, node))?.syncError;
  };

  const getNodeDiskObservation = (node: WorkspaceNode | null | undefined) => {
    if (!node || !activeGame?.id) return undefined;
    return switchRecords.get(nodePendingKey(activeGame.id, node))?.diskObservation;
  };

  return {
    isPending,
    isNodePending,
    getPendingDesiredEnabled,
    getNodeSyncError,
    getNodeDiskObservation,
    toggleNode,
    setNodeEnabled,
    setFolderPathEnabled,
    resolveParentEnable,
    resolveDuplicateForceEnable,
    resolveDuplicateEnableOnly,
  };
}
