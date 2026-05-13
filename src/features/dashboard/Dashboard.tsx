import { Copy, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { DashboardPayload } from '../../types/dashboard';
import { useAppStore } from '../../stores/useAppStore';
import { useDashboardStats } from './hooks/useDashboardStats';
import { useActiveKeybindings } from './hooks/useActiveKeybindings';
import { useActiveGame } from '../../hooks/useActiveGame';
import { formatBytes } from '../../utils/formatters';
import { DashboardActivity } from './components/DashboardActivity';
import { DashboardCharts } from './components/DashboardCharts';
import { DashboardQuickActions } from './components/DashboardQuickActions';
import { DashboardStats } from './components/DashboardStats';
import {
  DashboardEmptyState,
  DashboardErrorState,
  DashboardLoadingState,
} from './components/DashboardStatusStates';

const EMPTY_STATS = {
  total_games: 0,
  total_mods: 0,
  enabled_mods: 0,
  disabled_mods: 0,
  total_size_bytes: 0,
  total_collections: 0,
};

export default function Dashboard() {
  const { t } = useTranslation(['dashboard', 'common']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const theme = useAppStore((state) => state.theme);
  const { data, isLoading, isError, refresh } = useDashboardStats();
  const { activeGame } = useActiveGame();
  const { keybindings, isLoading: keybindingsLoading } = useActiveKeybindings();

  if (isLoading) {
    return <DashboardLoadingState />;
  }

  if (isError || !data) {
    return <DashboardErrorState onRetry={refresh} />;
  }

  if (data.stats.total_games === 0) {
    return <DashboardEmptyState />;
  }

  const stats = data.stats || EMPTY_STATS;
  const duplicateWasteBytes = data.duplicate_waste_bytes ?? 0;
  const categoryDistribution = data.category_distribution ?? [];
  const gameDistribution = data.game_distribution ?? [];
  const recentMods: DashboardPayload['recent_mods'] = data.recent_mods ?? [];

  return (
    <div className="h-full overflow-y-auto bg-base-100">
      <div className="p-6 space-y-6 max-w-7xl mx-auto">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">{t('header.title')}</h1>
            <p className="text-sm text-base-content/50">
              {t('header.subtitle')}
              {activeGameId ? ` • ${activeGameId}` : ''}
            </p>
          </div>
          <button
            onClick={refresh}
            className="btn btn-ghost btn-sm gap-2 text-base-content/60 hover:text-base-content"
            aria-label={t('header.refresh')}
          >
            <RefreshCw size={16} />
            {t('header.refresh')}
          </button>
        </div>

        <DashboardQuickActions activeGameId={activeGameId} setWorkspaceView={setWorkspaceView} />
        <DashboardStats stats={stats} />

        {duplicateWasteBytes > 0 && (
          <div
            role="alert"
            className="alert alert-warning alert-soft alert-horizontal cursor-pointer hover:brightness-95 transition-all"
            onClick={() => setWorkspaceView('storage-optimizer')}
          >
            <Copy size={20} />
            <div>
              <h3 className="font-bold">{t('waste.title')}</h3>
              <p className="text-sm">
                {t('waste.subtitle', { size: formatBytes(duplicateWasteBytes) })}
              </p>
            </div>
          </div>
        )}

        <DashboardCharts
          categoryDistribution={categoryDistribution}
          gameDistribution={gameDistribution}
          theme={theme}
        />
        <DashboardActivity
          activeGame={activeGame}
          keybindings={keybindings}
          keybindingsLoading={keybindingsLoading}
          recentMods={recentMods}
        />
      </div>
    </div>
  );
}
