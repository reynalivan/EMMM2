import { useCallback, useMemo, useState } from 'react';
import { join } from '@tauri-apps/api/path';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { toast } from '../../../stores/useToastStore';
import type {
  WorkspaceExplorerNode,
  WorkspaceNode,
  WorkspaceObjectNode,
} from '../../../types/workspace';
import { applyRuntimeEffects } from '../optimistic/applyOptimisticEffects';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { dispatchWorkspaceRuntimeEvent } from '../state/workspaceStoreBridge';
import {
  applyEnableOnlyThisEffects,
  applyWorkspaceSwitchEffects,
  buildExplorerSwitchEffectDescriptor,
  buildNodePendingKey,
  buildSwitchRefreshDescriptor,
  executeWorkspaceSwitch,
  isWorkspaceObjectNode,
  togglePendingKey,
  type WorkspaceSwitchSurface,
} from './workspaceSwitchOps';

export type { WorkspaceSwitchSurface } from './workspaceSwitchOps';

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
    ) => {
      if (!activeGame?.id) {
        return null;
      }

      if (desiredEnabled && node.switch_state === 'blocked_by_ancestor') {
        return null;
      }

      const result = await executeWorkspaceSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'mod_path',
          value: node.path,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        origin_surface: surface,
      });
      if (!result) {
        return null;
      }

      if (result.status === 'requires_duplicate_resolution') {
        dispatchWorkspaceRuntimeEvent({
          type: 'DIALOG_OPENED',
          dialog: {
            kind: 'modDuplicateWarning',
            folder: node,
            duplicates: result.duplicates,
          },
        });
        return null;
      }

      const nextPath = result.primary_path;
      if (!nextPath) {
        return null;
      }

      applyRuntimeEffects(queryClient, buildExplorerSwitchEffectDescriptor(node, result.impact));
      await publishRuntimeDescriptor(
        queryClient,
        buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
        'active',
      );

      return nextPath;
    },
    [activeGame, queryClient],
  );

  const setObjectNodeEnabled = useCallback(
    async (node: WorkspaceObjectNode, desiredEnabled: boolean, surface: WorkspaceSwitchSurface) => {
      // Explicit object enable/disable stays in Workspace Switch.
      // This path must not rely on Disk Reconcile or mod-toggle semantics.
      if (!activeGame) {
        return null;
      }

      const targetPath = await join(activeGame.mod_path, node.folder_path);
      const result = await executeWorkspaceSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'object_id',
          value: node.id,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        origin_surface: surface,
      });

      if (!result?.primary_path) {
        return null;
      }

      const nextPath = result.primary_path;
      await applyWorkspaceSwitchEffects(queryClient, result, targetPath, 'objectSwitch');
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
    ) => {
      const pendingKey = buildNodePendingKey(node);
      markPending(pendingKey, true);

      try {
        if (isWorkspaceObjectNode(node)) {
          return await setObjectNodeEnabled(node, desiredEnabled, surface);
        }

        return await setExplorerNodeEnabled(node, desiredEnabled, surface);
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
        const result = await executeWorkspaceSwitch({
          game_id: activeGame.id,
          target: {
            kind: 'mod_path',
            value: path,
          },
          desired_enabled: desiredEnabled,
          resolution: 'normal',
          origin_surface: 'folder_grid',
        });
        const nextPath = result?.primary_path;
        if (!result || !nextPath) {
          return null;
        }

        await applyWorkspaceSwitchEffects(queryClient, result, path, 'folderSwitch');

        return nextPath;
      } finally {
        markPending(pendingKey, false);
      }
    },
    [activeGame, markPending, queryClient],
  );

  const resolveDuplicateForceEnable = useCallback(
    async (folder: Pick<WorkspaceExplorerNode, 'path'> | null) => {
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
        origin_surface: 'folder_grid',
      });
      if (!result?.primary_path) {
        return null;
      }

      await applyWorkspaceSwitchEffects(queryClient, result, folder.path, 'folderSwitch');
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, queryClient],
  );

  const resolveDuplicateEnableOnly = useCallback(
    async (folder: Pick<WorkspaceExplorerNode, 'path'> | null) => {
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
        origin_surface: 'folder_grid',
      });
      if (!result) {
        return null;
      }

      await applyEnableOnlyThisEffects(queryClient, result);
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, queryClient],
  );

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
    resolveDuplicateForceEnable,
    resolveDuplicateEnableOnly,
  };
}
