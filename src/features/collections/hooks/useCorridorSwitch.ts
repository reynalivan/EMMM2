/**
 * useCorridorSwitch — Mutation hook for switching corridor mode.
 *
 * Replaces: useAppStore.setSafeMode + useSafeModeToggle.setSafeModeWithToast chain.
 * Calls switch_corridor → invalidates corridor + collections + objects + mod-folders.
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import { collectionKeys, corridorKeys } from '../queryKeys';
import { commands } from '../../../lib/bindings';
import { formatAppError } from '../../../lib/appError';
import {
  publishQueryInvalidations,
  publishRuntimeDescriptor,
} from '../../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../../workspace-runtime/optimistic/descriptorBuilders';

export function useCorridorSwitch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, targetSafe }: { gameId: string; targetSafe: boolean }) =>
      commands.switchCorridor({ gameId, targetSafe }),

    onSuccess: async (result, { gameId, targetSafe }) => {
      const previousSafe = useAppStore.getState().safeMode;
      // Update zustand store
      const activeSafe = result.active_safe;
      useAppStore.setState({
        safeMode: activeSafe,
        gridSelection: new Set(),
        selectedObjectFolderPath: null,
        selectedModPath: null,
        explorerSubPath: undefined,
        currentPath: [],
        mobileActivePane: 'sidebar',
      });

      // Invalidate all caches that depend on corridor state
      await publishRuntimeDescriptor(
        queryClient,
        buildRuntimeMutationDescriptor('corridorState'),
        'active',
      );
      const corridorState = await commands.getCorridorState({
        gameId,
        isSafe: activeSafe,
      });
      queryClient.setQueryData(corridorKeys.state(gameId, activeSafe), corridorState);
      await Promise.all([
        publishQueryInvalidations(
          queryClient,
          [collectionKeys.list(gameId, activeSafe), collectionKeys.list(gameId, previousSafe)],
          'active',
        ),
        publishQueryInvalidations(
          queryClient,
          [corridorKeys.switchPreview(gameId, previousSafe, targetSafe)],
          'all',
        ),
      ]);

      // Build toast message
      const label = activeSafe ? 'SAFE Mode Enabled' : 'UNSAFE Mode Enabled';
      const parts: string[] = [];
      if (result.mods_disabled > 0) parts.push(`Disabled ${result.mods_disabled}`);
      if (result.mods_restored > 0) parts.push(`Restored ${result.mods_restored}`);
      const detail = parts.length > 0 ? ` — ${parts.join(', ')} mod(s)` : '';
      const warningDetail =
        result.warnings && result.warnings.length > 0
          ? ` — ${result.warnings.length} warning(s)`
          : '';
      toast.success(`${label}${detail}${warningDetail}`);
    },

    onError: (err) => {
      toast.error(formatAppError(err));
    },
  });
}
