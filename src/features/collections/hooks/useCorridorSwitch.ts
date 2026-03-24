/**
 * useCorridorSwitch — Mutation hook for switching corridor mode.
 *
 * Replaces: useAppStore.setSafeMode + useSafeModeToggle.setSafeModeWithToast chain.
 * Calls switch_corridor → invalidates corridor + collections + objects + mod-folders.
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import { corridorKeys, collectionKeys } from '../queryKeys';
import { commands } from '../../../lib/bindings';

export function useCorridorSwitch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ gameId, targetSafe }: { gameId: string; targetSafe: boolean }) =>
      commands.switchCorridor({ gameId, targetSafe }),

    onSuccess: (result, { targetSafe }) => {
      // Update zustand store
      useAppStore.setState({
        safeMode: targetSafe,
        gridSelection: new Set(),
        selectedObjectFolderPath: null,
      });

      // Invalidate all caches that depend on corridor state
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
      queryClient.invalidateQueries({ queryKey: collectionKeys.all });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });

      // Build toast message
      const label = targetSafe ? 'SAFE Mode Enabled' : 'UNSAFE Mode Enabled';
      const parts: string[] = [];
      if (result.mods_disabled > 0) parts.push(`Disabled ${result.mods_disabled}`);
      if (result.mods_enabled > 0) parts.push(`Restored ${result.mods_enabled}`);
      const detail = parts.length > 0 ? ` — ${parts.join(', ')} mod(s)` : '';
      toast.success(`${label}${detail}`);
    },

    onError: (err) => {
      toast.error(String(err));
    },
  });
}
