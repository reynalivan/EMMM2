import { useCallback, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
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

export function useWorkspaceSwitchActions() {
  const { t } = useTranslation(['common', 'objects']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const [pendingKeys, setPendingKeys] = useState<Record<string, boolean>>({});

  const markPending = useCallback((key: string, pending: boolean) => {
    setPendingKeys((current) => togglePendingKey(current, key, pending));
  }, []);

  const setExplorerNodeEnabled = useCallback(
    async (
      node: WorkspaceExplorerNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceSwitchEffectsOptions,
    ) => {
      if (!activeGame?.id) {
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
        origin_surface: surface,
      };
      const result = await executeWorkspaceSwitch(input);
      if (!result) {
        return null;
      }

      if (result.status === 'requires_parent_enable' && result.parent_enable_requirement) {
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
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'modDuplicateWarning',
            folder: dialogFolder(node.path, node.name, node.id),
            duplicates: result.duplicates,
            enableDisabledAncestors: false,
          },
        });
        return null;
      }

      const nextPath = result.primary_path;
      if (!nextPath) {
        return null;
      }

      await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        ...options,
        gameId: activeGame.id,
      });

      return nextPath;
    },
    [activeGame, queryClient],
  );

  const setObjectNodeEnabled = useCallback(
    async (
      node: WorkspaceObjectNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options?: WorkspaceSwitchEffectsOptions,
    ) => {
      // Explicit object enable/disable stays in Workspace Switch.
      // This path must not rely on Disk Reconcile or mod-toggle semantics.
      if (!activeGame) {
        return null;
      }

      const result = await executeWorkspaceSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'object_id',
          value: node.id,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        enable_disabled_ancestors: false,
        origin_surface: surface,
      });

      if (!result?.primary_path) {
        return null;
      }

      const nextPath = result.primary_path;
      await applyWorkspaceSwitchEffects(queryClient, result, 'objectSwitch', {
        ...options,
        gameId: activeGame.id,
      });
      // A no-op switch changed nothing on disk — don't announce a change.
      if (result.status !== 'noop') {
        toast.success(
          t(desiredEnabled ? 'objects:toasts.enabled_one' : 'objects:toasts.disabled_one', {
            count: 1,
          }),
        );
      }

      return nextPath;
    },
    [activeGame, queryClient, t],
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
        if (isWorkspaceObjectNode(node)) {
          return await setObjectNodeEnabled(node, desiredEnabled, surface, options);
        }

        return await setExplorerNodeEnabled(node, desiredEnabled, surface, options);
      } finally {
        markPending(pendingKey, false);
      }
    },
    [markPending, setExplorerNodeEnabled, setObjectNodeEnabled],
  );

  const toggleNode = useCallback(
    async (node: WorkspaceNode, surface: WorkspaceSwitchSurface) => {
      const desiredEnabled = node.switch_state !== 'enabled';
      return setNodeEnabled(node, desiredEnabled, surface);
    },
    [setNodeEnabled],
  );

  const setFolderPathEnabled = useCallback(
    async (path: string, desiredEnabled: boolean) => {
      if (!activeGame?.id) {
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
          origin_surface: 'folder_grid',
        };
        const result = await executeWorkspaceSwitch(input);
        if (!result) {
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
              enableDisabledAncestors: false,
            },
          });
          return null;
        }

        const nextPath = result?.primary_path;
        if (!nextPath) {
          return null;
        }

        await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
          gameId: activeGame.id,
        });

        return nextPath;
      } finally {
        markPending(pendingKey, false);
      }
    },
    [activeGame, markPending, queryClient],
  );

  const resolveDuplicateForceEnable = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path'> | null,
      enableDisabledAncestors: boolean = false,
    ) => {
      if (!folder || !activeGame?.id) {
        return null;
      }

      const result = await executeWorkspaceSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: folder.path,
        },
        desired_enabled: true,
        resolution: 'force_enable',
        enable_disabled_ancestors: enableDisabledAncestors,
        origin_surface: 'folder_grid',
      });
      if (!result?.primary_path) {
        return null;
      }

      await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
      });
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, queryClient],
  );

  const resolveDuplicateEnableOnly = useCallback(
    async (
      folder: Pick<WorkspaceExplorerNode, 'path'> | null,
      enableDisabledAncestors: boolean = false,
    ) => {
      if (!folder || !activeGame?.id) {
        return null;
      }

      const result = await executeWorkspaceSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: folder.path,
        },
        desired_enabled: true,
        resolution: 'enable_only_this',
        enable_disabled_ancestors: enableDisabledAncestors,
        origin_surface: 'folder_grid',
      });
      if (!result) {
        return null;
      }

      await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
        gameId: activeGame.id,
      });
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, queryClient],
  );

  const resolveParentEnable = useCallback(async () => {
    const dialogState = getWorkspaceRuntimeState().dialogState;
    if (dialogState.kind !== 'folderEnableParent') {
      return null;
    }

    const result = await executeWorkspaceSwitch({
      ...dialogState.resumeInput,
      enable_disabled_ancestors: true,
    });
    if (!result) {
      return null;
    }

    if (result.status === 'requires_duplicate_resolution') {
      dispatchWorkspaceRuntimeEvent({
        type: 'DIALOG_OPENED',
        dialog: {
          kind: 'modDuplicateWarning',
          folder: dialogState.folder,
          duplicates: result.duplicates,
          enableDisabledAncestors: true,
        },
      });
      return null;
    }
    if (!result.primary_path || !activeGame?.id) {
      return null;
    }

    await applyWorkspaceSwitchEffects(queryClient, result, 'folderSwitch', {
      gameId: activeGame.id,
    });
    dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'folderEnableParent' });
    return result.primary_path;
  }, [activeGame, queryClient]);

  const isPending = useMemo(() => Object.keys(pendingKeys).length > 0, [pendingKeys]);

  const isNodePending = useCallback(
    (node: WorkspaceNode | null | undefined) => {
      if (!node) {
        return false;
      }

      return !!pendingKeys[buildNodePendingKey(node)];
    },
    [pendingKeys],
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
