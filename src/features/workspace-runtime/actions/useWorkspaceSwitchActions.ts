import { useCallback, useRef, useState } from 'react';
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
} from '@/entities/workspace';
import type { ModFolder } from '@/entities/game-object';
import {
  dispatchWorkspaceRuntimeEvent,
  getWorkspaceRuntimeState,
} from '../state/workspaceStoreBridge';
import {
  applyWorkspaceSwitchEffects,
  buildNodePendingKey,
  executeWorkspaceSwitch,
  isWorkspaceGameCurrent,
  isWorkspaceObjectNode,
  togglePendingKey,
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

interface WorkspaceNodeSwitchOptions extends WorkspaceSwitchEffectsOptions {
  /** Skips transient feedback when a newer toggle intent has superseded this result. */
  shouldNotifyResult?: () => boolean;
}

interface LatestNodeToggleIntent {
  desiredEnabled: boolean;
  node: WorkspaceNode;
  surface: WorkspaceSwitchSurface;
  waiters: Array<{
    resolve: (path: string | null) => void;
    reject: (error: unknown) => void;
  }>;
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
  const [pendingKeys, setPendingKeys] = useState<Record<string, boolean>>({});
  const latestNodeToggleIntents = useRef(new Map<string, LatestNodeToggleIntent>());

  const markPending = useCallback((key: string, pending: boolean) => {
    setPendingKeys((current) => togglePendingKey(current, key, pending));
  }, []);

  const setExplorerNodeEnabled = useCallback(
    async (
      node: WorkspaceExplorerNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ) => {
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
      const result = await executeWorkspaceSwitch(input);
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

      applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        ...options,
        gameId: activeGame.id,
      });
      if (result.duplicates.length > 0 && options?.shouldNotifyResult?.() !== false) {
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

      return nextPath;
    },
    [activeGame, activationBlocksMutations, queryClient],
  );

  const setObjectNodeEnabled = useCallback(
    async (
      node: WorkspaceObjectNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ) => {
      // Explicit object enable/disable stays in Workspace Switch.
      // This path must not rely on Disk Reconcile or mod-toggle semantics.
      if (!activeGame) {
        return null;
      }
      if (activationBlocksMutations) {
        return null;
      }

      const gameId = activeGame.id;
      const result = await executeWorkspaceSwitch({
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
      });

      if (!result?.primary_path || !isWorkspaceGameCurrent(gameId)) {
        return null;
      }

      const nextPath = result.primary_path;
      if (result.status !== 'noop') {
        applyWorkspaceSwitchEffects(queryClient, result, 'objectSwitch', {
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

      return nextPath;
    },
    [activeGame, activationBlocksMutations, queryClient, t],
  );

  const executeNodeEnabled = useCallback(
    async (
      node: WorkspaceNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceNodeSwitchOptions,
    ) => {
      if (isWorkspaceObjectNode(node)) {
        return setObjectNodeEnabled(node, desiredEnabled, surface, options);
      }

      return setExplorerNodeEnabled(node, desiredEnabled, surface, options);
    },
    [setExplorerNodeEnabled, setObjectNodeEnabled],
  );

  const setNodeEnabled = useCallback(
    async (
      node: WorkspaceNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceSwitchEffectsOptions,
    ) => {
      const pendingKey = buildNodePendingKey(node);
      markPending(pendingKey, true);

      try {
        return await executeNodeEnabled(node, desiredEnabled, surface, options);
      } finally {
        markPending(pendingKey, false);
      }
    },
    [executeNodeEnabled, markPending],
  );

  const toggleNode = useCallback(
    (node: WorkspaceNode, surface: WorkspaceSwitchSurface) => {
      const pendingKey = buildNodePendingKey(node);
      const existingIntent = latestNodeToggleIntents.current.get(pendingKey);
      if (existingIntent) {
        existingIntent.desiredEnabled = !existingIntent.desiredEnabled;
        existingIntent.node = node;
        existingIntent.surface = surface;
        return new Promise<string | null>((resolve, reject) => {
          existingIntent.waiters.push({ resolve, reject });
        });
      }

      const intent: LatestNodeToggleIntent = {
        desiredEnabled: node.switch_state !== 'enabled',
        node,
        surface,
        waiters: [],
      };
      latestNodeToggleIntents.current.set(pendingKey, intent);

      const completion = new Promise<string | null>((resolve, reject) => {
        intent.waiters.push({ resolve, reject });
      });

      const runLatestIntent = async () => {
        try {
          while (true) {
            const desiredEnabled = intent.desiredEnabled;
            const intentNode = intent.node;
            const nextPath = await executeNodeEnabled(intentNode, desiredEnabled, intent.surface, {
              shouldNotifyResult: () => {
                const currentIntent = latestNodeToggleIntents.current.get(pendingKey);
                return currentIntent === intent && currentIntent.desiredEnabled === desiredEnabled;
              },
            });

            if (!nextPath || intent.desiredEnabled === desiredEnabled) {
              for (const waiter of intent.waiters) {
                waiter.resolve(nextPath);
              }
              return;
            }

            if (!isWorkspaceObjectNode(intentNode)) {
              intent.node = { ...intentNode, path: nextPath };
            }
          }
        } catch (error) {
          for (const waiter of intent.waiters) {
            waiter.reject(error);
          }
        } finally {
          if (latestNodeToggleIntents.current.get(pendingKey) === intent) {
            latestNodeToggleIntents.current.delete(pendingKey);
          }
          markPending(pendingKey, false);
        }
      };

      markPending(pendingKey, true);
      void runLatestIntent();
      return completion;
    },
    [executeNodeEnabled, markPending],
  );

  const setFolderPathEnabled = useCallback(
    async (path: string, desiredEnabled: boolean) => {
      if (!activeGame?.id || activationBlocksMutations) {
        return null;
      }

      const pendingKey = `folder:${path}`;
      markPending(pendingKey, true);

      try {
        const input: WorkspaceSwitchInput = {
          game_id: activeGame.id,
          target: {
            kind: 'mod_path',
            value: path,
          },
          desired_enabled: desiredEnabled,
          resolution: 'normal',
          enable_disabled_ancestors: false,
          parent_enable_confirmation: null,
          origin_surface: 'folder_grid',
        };
        const result = await executeWorkspaceSwitch(input);
        if (!result || !isWorkspaceGameCurrent(input.game_id)) {
          return null;
        }

        const folder = dialogFolder(path);
        if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
          dispatchWorkspaceRuntimeEvent({
            type: 'DIALOG_OPENED',
            dialog: {
              kind: 'folderEnableParent',
              folder,
              requirement: result.parent_enable_requirement,
              resumeInput: input,
            },
          });
          return null;
        }
        if (result.status === 'requires_duplicate_resolution') {
          dispatchWorkspaceRuntimeEvent({
            type: 'DIALOG_OPENED',
            dialog: {
              kind: 'modDuplicateWarning',
              folder,
              duplicates: result.duplicates,
              requiresResolution: true,
              enableDisabledAncestors: false,
              parentEnableConfirmation: null,
            },
          });
          return null;
        }

        const nextPath = result?.primary_path;
        if (!nextPath) {
          return null;
        }

        applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
          gameId: activeGame.id,
        });
        if (result.duplicates.length > 0) {
          dispatchWorkspaceRuntimeEvent({
            type: 'DIALOG_OPENED',
            dialog: {
              kind: 'modDuplicateWarning',
              folder: dialogFolder(nextPath),
              duplicates: result.duplicates,
              requiresResolution: false,
              enableDisabledAncestors: false,
              parentEnableConfirmation: null,
            },
          });
        }

        return nextPath;
      } finally {
        markPending(pendingKey, false);
      }
    },
    [activeGame, activationBlocksMutations, markPending, queryClient],
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

  const isNodePending = useCallback(
    (node: WorkspaceNode | null | undefined) => {
      if (!node) {
        return false;
      }

      return activationBlocksMutations || !!pendingKeys[buildNodePendingKey(node)];
    },
    [activationBlocksMutations, pendingKeys],
  );

  return {
    isPending,
    isNodePending,
    toggleNode,
    setNodeEnabled,
    setFolderPathEnabled,
    resolveParentEnable,
    resolveDuplicateForceEnable,
    resolveDuplicateEnableOnly,
  };
}
