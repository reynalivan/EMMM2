import { useCallback } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { settingsQueryOptions } from '@/entities/settings';
import { toast } from '@/shared/ui/toast';
import { getModViewerLaunchPolicy } from './modViewerLaunchPolicy';
import { recordModViewerLaunchSnapshot } from './modViewerExternalReviewState';

export function useModViewerLaunch(folder: WorkspaceExplorerNode | null | undefined) {
  const { t } = useTranslation('grid');
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const { data: settings } = useQuery(settingsQueryOptions);
  const policy = getModViewerLaunchPolicy(
    settings?.external_tools?.mod_viewer_executable,
    activeGame?.game_type,
  );

  const launch = useCallback(async () => {
    if (!folder || !activeGame || !policy.visible) {
      return;
    }

    try {
      const receipt = await commands.launchModViewer(activeGame.id, folder.path);
      recordModViewerLaunchSnapshot(queryClient, receipt);
    } catch (error) {
      toast.error(t('context.mod_viewer_launch_failed', { error: formatAppError(error) }));
    }
  }, [activeGame, folder, policy.visible, queryClient, t]);

  const actionLabel = t(
    policy.experimental ? 'context.open_mod_viewer_experimental' : 'context.open_mod_viewer',
  );
  const tooltip = policy.experimental
    ? t('context.open_mod_viewer_experimental_tooltip')
    : actionLabel;

  return {
    ...policy,
    actionLabel,
    tooltip,
    launch,
  };
}
