import { useCallback, useMemo, useState } from 'react';
import { join } from '@tauri-apps/api/path';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../../hooks/useActiveGame';
import {
  patchObjectRootSwitchState,
  restoreObjectListQueries,
  snapshotObjectListQueries,
} from '../../../hooks/objectQueryCache';
import { commands } from '../../../lib/bindings';
import { formatAppError, extractFileInUsePayload } from '../../../lib/appError';
import { toast } from '../../../stores/useToastStore';
import type {
  WorkspaceImpact,
  WorkspaceExplorerNode,
  WorkspaceNode,
  WorkspaceObjectNode,
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
} from '../../../types/workspace';
import { toggleDisabledInPath } from '../../../lib/disabledPrefix';
import { thumbnailKeys } from '../../../hooks/useThumbnail';
import { updateFolderCache } from '../../../hooks/folderCache';
import { applyRuntimeEffects } from '../optimistic/applyOptimisticEffects';
import {
  buildRuntimeRefreshDescriptor,
  buildRuntimeMutationDescriptor,
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
  buildObjectCountDeltaDescriptor,
} from '../optimistic/descriptorBuilders';
import { mergeRuntimeEffectDescriptors } from '../optimistic/descriptor';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { dispatchWorkspaceRuntimeEvent } from '../state/workspaceStoreBridge';
import {
  openWorkspaceConflictDialog,
  openWorkspaceFileInUseDialog,
} from '../state/workspaceDialogs';

export type WorkspaceSwitchSurface =
  | 'folder_grid'
  | 'preview'
  | 'object_list'
  | 'collections'
  | 'corridor';

interface PathSwitchOptions {
  syncExplorerPath: boolean;
}

function isWorkspaceObjectNode(node: WorkspaceNode): node is WorkspaceObjectNode {
  return node.node_kind === 'object';
}

function parseRenameConflict(error: unknown) {
  const raw = error instanceof Error ? error.message : String(error);
  if (!raw.includes('"type":"RenameConflict"')) {
    return null;
  }

  try {
    return JSON.parse(raw) as {
      type: 'RenameConflict';
      attempted_target: string;
      existing_path: string;
      base_name: string;
    };
  } catch {
    return null;
  }
}

function buildNodePendingKey(node: WorkspaceNode): string {
  if (isWorkspaceObjectNode(node)) {
    return `object:${node.id}`;
  }

  return `folder:${node.path}`;
}

function buildSwitchRefreshDescriptor(
  impact: WorkspaceImpact | null | undefined,
  fallbackClass: 'folderSwitch' | 'objectSwitch',
) {
  if (!impact || impact.refresh_scopes.length === 0) {
    return buildRuntimeMutationDescriptor(fallbackClass);
  }

  return buildRuntimeRefreshDescriptor(impact.refresh_scopes);
}

function normalizeWorkspaceSwitchPath(path: string): string {
  return path.replace(/\\/g, '/');
}

function stripModsRoot(path: string, modsPath: string): string {
  const normalizedPath = normalizeWorkspaceSwitchPath(path);
  const normalizedModsPath = normalizeWorkspaceSwitchPath(modsPath);
  if (normalizedPath === normalizedModsPath) {
    return normalizedPath;
  }

  const prefix = `${normalizedModsPath}/`;
  if (!normalizedPath.startsWith(prefix)) {
    return normalizedPath;
  }

  return normalizedPath.slice(prefix.length);
}

export function useWorkspaceSwitchActions() {
  const { t } = useTranslation(['common', 'objects']);
  const queryClient = useQueryClient();
  const { activeGame } = useActiveGame();
  const [pendingKeys, setPendingKeys] = useState<Record<string, boolean>>({});

  const markPending = useCallback((key: string, pending: boolean) => {
    setPendingKeys((current) => {
      if (pending) {
        return { ...current, [key]: true };
      }

      if (!current[key]) {
        return current;
      }

      const next = { ...current };
      delete next[key];
      return next;
    });
  }, []);

  const openConflictError = useCallback((error: unknown) => {
    const renameConflict = parseRenameConflict(error);
    if (renameConflict) {
      openWorkspaceConflictDialog(renameConflict);
      return true;
    }

    const fileInUse = extractFileInUsePayload(error);
    if (fileInUse) {
      openWorkspaceFileInUseDialog({
        path: fileInUse.path,
        processes: fileInUse.processes,
      });
      return true;
    }

    return false;
  }, []);

  const executeSwitch = useCallback(
    async (input: WorkspaceSwitchInput): Promise<WorkspaceSwitchResult | null> => {
      try {
        return await commands.executeWorkspaceSwitch({ input });
      } catch (error) {
        if (openConflictError(error)) {
          return null;
        }

        toast.error(formatAppError(error));
        return null;
      }
    },
    [openConflictError],
  );

  const setExplorerNodeEnabled = useCallback(
    async (
      node: WorkspaceExplorerNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      _options: PathSwitchOptions,
    ) => {
      if (!activeGame?.id) {
        return null;
      }

      if (desiredEnabled && node.switch_state === 'blocked_by_ancestor') {
        return null;
      }

      const result = await executeSwitch({
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

      updateFolderCache(queryClient, [node.path], (folder) => ({
        ...folder,
        path: nextPath,
        is_enabled: desiredEnabled,
      }));
      const countDelta =
        node.node_kind === 'terminal_mod' &&
        node.owner_object_id &&
        node.is_enabled !== desiredEnabled
          ? buildObjectCountDeltaDescriptor(node.owner_object_id, desiredEnabled ? 1 : -1, [])
          : null;
      applyRuntimeEffects(
        queryClient,
        mergeRuntimeEffectDescriptors(
          buildQueryRemovalDescriptor([thumbnailKeys.folder(node.path)], []),
          buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
          ...(countDelta ? [countDelta] : []),
        ),
      );
      await publishRuntimeDescriptor(
        queryClient,
        buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
        'active',
      );

      return nextPath;
    },
    [activeGame, executeSwitch, queryClient],
  );

  const setObjectNodeEnabled = useCallback(
    async (node: WorkspaceObjectNode, desiredEnabled: boolean, surface: WorkspaceSwitchSurface) => {
      // Explicit object enable/disable stays in Workspace Switch.
      // This path must not rely on Disk Reconcile or mod-toggle semantics.
      if (!activeGame) {
        return null;
      }

      const previousQueries = snapshotObjectListQueries(queryClient);
      const targetPath = await join(activeGame.mod_path, node.folder_path);
      patchObjectRootSwitchState(queryClient, {
        objectId: node.id,
        folderPath: toggleDisabledInPath(node.folder_path, desiredEnabled),
        enabled: desiredEnabled,
      });

      const result = await executeSwitch({
        game_id: activeGame.id,
        target: {
          kind: 'object_id',
          value: node.id,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        origin_surface: surface,
      });

      if (!result) {
        restoreObjectListQueries(queryClient, previousQueries);
        return null;
      }

      const nextPath = result.primary_path;
      if (!nextPath) {
        restoreObjectListQueries(queryClient, previousQueries);
        return null;
      }
      patchObjectRootSwitchState(queryClient, {
        objectId: node.id,
        folderPath: stripModsRoot(nextPath, activeGame.mod_path),
        enabled: desiredEnabled,
      });

      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor([thumbnailKeys.folder(targetPath)], []),
      );
      if (nextPath !== targetPath) {
        applyRuntimeEffects(
          queryClient,
          buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
        );
      }
      await publishRuntimeDescriptor(
        queryClient,
        buildSwitchRefreshDescriptor(result.impact, 'objectSwitch'),
        'active',
      );
      toast.success(
        t(desiredEnabled ? 'objects:toasts.enabled_one' : 'objects:toasts.disabled_one', {
          count: 1,
        }),
      );

      return nextPath;
    },
    [activeGame, executeSwitch, queryClient, t],
  );

  const setNodeEnabled = useCallback(
    async (
      node: WorkspaceNode,
      desiredEnabled: boolean,
      surface: WorkspaceSwitchSurface,
      options: PathSwitchOptions,
    ) => {
      const pendingKey = buildNodePendingKey(node);
      markPending(pendingKey, true);

      try {
        if (isWorkspaceObjectNode(node)) {
          return await setObjectNodeEnabled(node, desiredEnabled, surface);
        }

        return await setExplorerNodeEnabled(node, desiredEnabled, surface, options);
      } finally {
        markPending(pendingKey, false);
      }
    },
    [markPending, setExplorerNodeEnabled, setObjectNodeEnabled],
  );

  const toggleNode = useCallback(
    async (node: WorkspaceNode, surface: WorkspaceSwitchSurface, options: PathSwitchOptions) => {
      const desiredEnabled = node.switch_state !== 'enabled';
      return setNodeEnabled(node, desiredEnabled, surface, options);
    },
    [setNodeEnabled],
  );

  const setFolderPathEnabled = useCallback(
    async (path: string, desiredEnabled: boolean, _options: PathSwitchOptions) => {
      if (!activeGame?.id) {
        return null;
      }

      const pendingKey = `folder:${path}`;
      markPending(pendingKey, true);

      try {
        const result = await executeSwitch({
          game_id: activeGame.id,
          target: {
            kind: 'mod_path',
            value: path,
          },
          desired_enabled: desiredEnabled,
          resolution: 'normal',
          origin_surface: 'folder_grid',
        });
        if (!result) {
          return null;
        }

        const nextPath = result.primary_path;
        if (!nextPath) {
          return null;
        }

        updateFolderCache(queryClient, [path], (folder) => ({
          ...folder,
          path: nextPath,
          is_enabled: desiredEnabled,
        }));
        applyRuntimeEffects(
          queryClient,
          buildQueryRemovalDescriptor([thumbnailKeys.folder(path)], []),
        );

        if (nextPath !== path) {
          applyRuntimeEffects(
            queryClient,
            buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
          );
        }
        await publishRuntimeDescriptor(
          queryClient,
          buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
          'active',
        );

        return nextPath;
      } finally {
        markPending(pendingKey, false);
      }
    },
    [activeGame, executeSwitch, markPending, queryClient],
  );

  const resolveDuplicateForceEnable = useCallback(
    async (folder: Pick<WorkspaceExplorerNode, 'path'> | null) => {
      if (!folder || !activeGame?.id) {
        return null;
      }

      const result = await executeSwitch({
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

      applyRuntimeEffects(
        queryClient,
        buildQueryRemovalDescriptor([thumbnailKeys.folder(folder.path)], []),
      );
      if (result.primary_path !== folder.path) {
        applyRuntimeEffects(
          queryClient,
          buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
        );
      }
      await publishRuntimeDescriptor(
        queryClient,
        buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
        'active',
      );
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, executeSwitch, queryClient],
  );

  const resolveDuplicateEnableOnly = useCallback(
    async (folder: Pick<WorkspaceExplorerNode, 'path'> | null) => {
      if (!folder || !activeGame?.id) {
        return null;
      }

      const result = await executeSwitch({
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

      if (result.changed_folder_paths.length > 0) {
        applyRuntimeEffects(
          queryClient,
          buildQueryRemovalDescriptor(
            result.changed_folder_paths.map((path) => thumbnailKeys.folder(path)),
            [],
          ),
        );
      }
      applyRuntimeEffects(
        queryClient,
        buildWorkspacePathRewritesDescriptor(result.impact.rewrites, []),
      );
      await publishRuntimeDescriptor(
        queryClient,
        buildSwitchRefreshDescriptor(result.impact, 'folderSwitch'),
        'active',
      );
      dispatchWorkspaceRuntimeEvent({ type: 'DIALOG_CLOSED', kind: 'modDuplicateWarning' });
      return result.primary_path;
    },
    [activeGame, executeSwitch, queryClient],
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
