import { useCallback, useSyncExternalStore } from 'react';
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
import { identityPathKey } from '@/shared/lib/pathKey';
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
  type WorkspaceSwitchSurface,
  type WorkspaceSwitchEffectsOptions,
} from './workspaceSwitchOps';

export type { WorkspaceSwitchSurface } from './workspaceSwitchOps';

function dialogFolder(
  path: string,
  name?: string,
  id: string | null = null,
): Pick<ModFolder, 'id' | 'path' | 'name'> {
  const segments = path.replace(/\\/g, '/').split('/').filter(Boolean);
  const fallbackName = segments[segments.length - 1] ?? path;
  return { id, path, name: name ?? fallbackName };
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
}

interface WorkspaceNodeSwitchOutcome {
  path: string;
  settled: Promise<void>;
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
  desiredEnabled: boolean;
  pendingRevision: number;
  completion: Promise<string | null>;
  resolve: (path: string | null) => void;
  reject: (error: unknown) => void;
}

const pendingKeys = new Set<string>();
const pendingDesiredEnabled = new Map<string, boolean>();
const pendingDesiredVersions = new Map<string, number>();
const pendingNodeOverrides = new Map<string, WorkspaceNode>();
const latestNodeToggleIntents = new Map<string, LatestNodeToggleIntent>();
const latestFolderPathIntents = new Map<string, LatestFolderPathIntent>();
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

function folderPathPendingKey(gameId: string, path: string): string {
  return scopedPendingKey(gameId, `folder:${identityPathKey(path) ?? path}`);
}

function nodePendingKey(gameId: string, node: WorkspaceNode): string {
  const ownKey = scopedPendingKey(gameId, buildNodePendingKey(node));
  if (isWorkspaceObjectNode(node)) {
    return ownKey;
  }
  const pathKey = folderPathPendingKey(gameId, node.path);
  if (
    latestNodeToggleIntents.has(ownKey) &&
    (pendingDesiredVersions.get(ownKey) ?? 0) >= (pendingDesiredVersions.get(pathKey) ?? 0)
  ) {
    return ownKey;
  }
  return pendingDesiredEnabled.has(pathKey) ? pathKey : ownKey;
}

function markPending(key: string, pending: boolean): void {
  if (pending) {
    pendingKeys.add(key);
  } else {
    pendingKeys.delete(key);
  }
  publishPendingSnapshot();
}

function setPendingDesired(key: string, desiredEnabled: boolean): number {
  const revision = nextWorkspaceIntentRevision();
  pendingDesiredVersions.set(key, revision);
  pendingDesiredEnabled.set(key, desiredEnabled);
  publishPendingSnapshot();
  return revision;
}

function clearPendingDesired(key: string, expectedRevision: number): void {
  if (pendingDesiredVersions.get(key) !== expectedRevision) {
    return;
  }
  pendingDesiredVersions.delete(key);
  pendingNodeOverrides.delete(key);
  pendingDesiredEnabled.delete(key);
  publishPendingSnapshot();
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
    pendingDesiredVersions.set(key, revision);
    pendingDesiredEnabled.set(key, desiredEnabled);
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
    if (pendingDesiredVersions.get(key) !== revision) continue;
    pendingDesiredVersions.delete(key);
    pendingDesiredEnabled.delete(key);
    changed = true;
  }
  if (changed) publishPendingSnapshot();
}

export function setFolderBulkPendingDesired(
  gameId: string,
  paths: Iterable<string>,
  desiredEnabled: boolean,
  revision: number,
): void {
  let changed = false;
  for (const path of paths) {
    const key = folderPathPendingKey(gameId, path);
    pendingDesiredVersions.set(key, revision);
    pendingDesiredEnabled.set(key, desiredEnabled);
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
    const key = folderPathPendingKey(gameId, path);
    if (pendingDesiredVersions.get(key) !== revision) continue;
    pendingDesiredVersions.delete(key);
    pendingDesiredEnabled.delete(key);
    changed = true;
  }
  if (changed) publishPendingSnapshot();
}

function withLatestFolderPath(node: WorkspaceNode, latestNode: WorkspaceNode | undefined) {
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
  useSyncExternalStore(
    subscribePendingSnapshot,
    getPendingSnapshotVersion,
    getPendingSnapshotVersion,
  );

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

      const input: WorkspaceSwitchInput = {
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: node.path,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        enable_disabled_ancestors: false,
        parent_enable_confirmation: null,
        origin_surface: surface,
      };
      const result = await executeWorkspaceSwitch(input, options?.intentRevision);
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
            folder: dialogFolder(node.path, node.name, node.id),
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
            folder: dialogFolder(node.path, node.name, node.id),
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
            folder: dialogFolder(nextPath, node.name, node.id),
            duplicates: result.duplicates,
            requiresResolution: false,
            enableDisabledAncestors: false,
            parentEnableConfirmation: null,
          },
        });
      }

      return { path: nextPath, settled };
    },
    [activeGame, activationBlocksMutations, queryClient],
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

      return { path: nextPath, settled };
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
      const pendingKey = nodePendingKey(activeGame.id, node);
      const pathIntent = latestFolderPathIntents.get(pendingKey);
      if (pathIntent) {
        pathIntent.desiredEnabled = desiredEnabled;
        pathIntent.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        admitWorkspaceIntentOverride(
          activeGame.id,
          [{ kind: 'mod_path', value: pathIntent.path }],
          pathIntent.pendingRevision,
        );
        return pathIntent.completion;
      }
      const existingIntent = latestNodeToggleIntents.get(pendingKey);
      if (existingIntent) {
        existingIntent.desiredEnabled = desiredEnabled;
        existingIntent.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        existingIntent.node = withLatestFolderPath(node, pendingNodeOverrides.get(pendingKey));
        existingIntent.surface = surface;
        existingIntent.options = options;
        existingIntent.execute = executeNodeEnabled;
        admitWorkspaceIntentOverride(
          activeGame.id,
          [
            isWorkspaceObjectNode(existingIntent.node)
              ? { kind: 'object_id', value: existingIntent.node.id }
              : { kind: 'mod_path', value: existingIntent.node.path },
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
        node: withLatestFolderPath(node, pendingNodeOverrides.get(pendingKey)),
        surface,
        options,
        execute: executeNodeEnabled,
        completion,
        resolve,
        reject,
      };
      latestNodeToggleIntents.set(pendingKey, intent);
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
              shouldNotifyResult: () =>
                latestNodeToggleIntents.get(pendingKey) === intent &&
                intent.pendingRevision === requestedRevision,
            });

            if (outcome?.path && !isWorkspaceObjectNode(intentNode)) {
              intent.node = { ...intentNode, path: outcome.path };
              pendingNodeOverrides.set(pendingKey, intent.node);
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
            const clearDesired = () => clearPendingDesired(pendingKey, completedRevision);
            void outcome.settled.then(clearDesired, () => undefined);
            intent.resolve(outcome.path);
            return;
          }
        } catch (error) {
          intent.reject(error);
        } finally {
          if (latestNodeToggleIntents.get(pendingKey) === intent) {
            latestNodeToggleIntents.delete(pendingKey);
          }
          markPending(pendingKey, false);
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
        pendingDesiredEnabled.get(pendingKey) ?? node.switch_state === 'enabled'
      );
      return submitNodeIntent(node, desiredEnabled, surface);
    },
    [activeGame, submitNodeIntent],
  );

  const setFolderPathEnabled = useCallback(
    (path: string, desiredEnabled: boolean): Promise<string | null> => {
      if (!activeGame?.id || activationBlocksMutations) {
        return Promise.resolve(null);
      }
      const gameId = activeGame.id;
      const identity = identityPathKey(path);
      for (const [key, nodeIntent] of latestNodeToggleIntents) {
        if (
          key.startsWith(`${gameId}:folder:`) &&
          !isWorkspaceObjectNode(nodeIntent.node) &&
          identityPathKey(nodeIntent.node.path) === identity
        ) {
          return submitNodeIntent(nodeIntent.node, desiredEnabled, 'folder_grid');
        }
      }

      const pendingKey = folderPathPendingKey(gameId, path);
      const existing = latestFolderPathIntents.get(pendingKey);
      if (existing) {
        existing.desiredEnabled = desiredEnabled;
        existing.pendingRevision = setPendingDesired(pendingKey, desiredEnabled);
        admitWorkspaceIntentOverride(
          gameId,
          [{ kind: 'mod_path', value: existing.path }],
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
        desiredEnabled,
        pendingRevision: setPendingDesired(pendingKey, desiredEnabled),
        completion,
        resolve,
        reject,
      };
      latestFolderPathIntents.set(pendingKey, intent);
      markPending(pendingKey, true);

      const runLatestIntent = async () => {
        let clearDesiredAfterRefresh = false;
        try {
          while (true) {
            const requestedEnabled = intent.desiredEnabled;
            const requestedRevision = intent.pendingRevision;
            const input: WorkspaceSwitchInput = {
              game_id: gameId,
              target: { kind: 'mod_path', value: intent.path },
              desired_enabled: requestedEnabled,
              resolution: 'normal',
              enable_disabled_ancestors: false,
              parent_enable_confirmation: null,
              origin_surface: 'folder_grid',
            };
            const result = await executeWorkspaceSwitch(input, requestedRevision);
            if (!isWorkspaceGameCurrent(gameId)) {
              intent.resolve(null);
              return;
            }
            if (result?.primary_path) {
              intent.path = result.primary_path;
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
                  folder: dialogFolder(intent.path),
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
                  folder: dialogFolder(intent.path),
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
            const settled = applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
              gameId,
            });
            const completedRevision = intent.pendingRevision;
            const clearDesired = () => clearPendingDesired(pendingKey, completedRevision);
            void settled.then(clearDesired, () => undefined);
            if (result.duplicates.length > 0 && requestedRevision === intent.pendingRevision) {
              dispatchWorkspaceRuntimeEvent({
                type: 'DIALOG_OPENED',
                dialog: {
                  kind: 'modDuplicateWarning',
                  folder: dialogFolder(result.primary_path),
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
          latestFolderPathIntents.delete(pendingKey);
          markPending(pendingKey, false);
          if (!clearDesiredAfterRefresh) {
            clearPendingDesired(pendingKey, intent.pendingRevision);
          }
        }
      };
      void runLatestIntent();
      return completion;
    },
    [activeGame?.id, activationBlocksMutations, queryClient, submitNodeIntent],
  );

  const resolveDuplicateForceEnable = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path'> | null,
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
            folder: dialogFolder(folder.path),
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
      applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
      });
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, activationBlocksMutations, queryClient],
  );

  const resolveDuplicateEnableOnly = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path'> | null,
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
            folder: dialogFolder(folder.path),
            requirement: result.parent_enable_requirement,
            resumeInput: input,
          },
        });
        return null;
      }

      showFolderDiskCommitToast(activeGame.id, result, true);
      applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
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
    applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
      gameId: activeGame.id,
    });
    dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'folderEnableParent' });
    if (result.duplicates.length > 0) {
      dispatchWorkspaceRuntimeEvent({
        type: 'DIALOG_OPENED',
        dialog: {
          kind: 'modDuplicateWarning',
          folder: dialogFolder(result.primary_path, dialogState.folder.name, dialogState.folder.id),
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
      (activeGame?.id !== undefined && pendingKeys.has(nodePendingKey(activeGame.id, node)))
    );
  };

  const getPendingDesiredEnabled = (node: WorkspaceNode | null | undefined) => {
    if (!node || !activeGame?.id) {
      return undefined;
    }
    return pendingDesiredEnabled.get(nodePendingKey(activeGame.id, node));
  };

  return {
    isPending,
    isNodePending,
    getPendingDesiredEnabled,
    toggleNode,
    setNodeEnabled,
    setFolderPathEnabled,
    resolveParentEnable,
    resolveDuplicateForceEnable,
    resolveDuplicateEnableOnly,
  };
}
