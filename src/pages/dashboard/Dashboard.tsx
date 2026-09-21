import { Copy } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { DashboardPayload } from './model/dashboard';
import { useAppStore } from '@/app/store';
import { useDashboardStats } from './hooks/useDashboardStats';
import { useActiveKeybindings } from './hooks/useActiveKeybindings';
import { useActiveGame } from '@/entities/game';
import { formatBytes } from '../../shared/lib/utils/formatters';
import { DashboardActivity } from './components/DashboardActivity';
import { DashboardCharts } from './components/DashboardCharts';
import { DashboardQuickActions } from './components/DashboardQuickActions';
import { DashboardStats } from './components/DashboardStats';
import { DashboardIdentitySuggestions } from './components/DashboardIdentitySuggestions';
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
  total_collections: 0,
};

export default function Dashboard() {
  const { t } = useTranslation(['dashboard', 'common']);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const setSettingsTab = useAppStore((state) => state.setSettingsTab);
  const activeGameId = useAppStore((state) => state.activeGameId);
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
  const dashboardContextKey = activeGameId ?? 'all-games';

  return (
    <div className="workspace-scroll-owner h-full overflow-y-auto bg-base-100/85">
      <div
        key={dashboardContextKey}
        className="dashboard-entrance max-w-7xl mx-auto space-y-6 p-6 pt-[calc(var(--workspace-topbar-height)+1.5rem)]"
      >
        <div className="dashboard-entrance__scope flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">{t('header.title')}</h1>
            <p className="text-sm text-base-content/50">
              {t('header.subtitle')}
              {/* The game's own name, not its internal slug. */}
              {activeGameId ? ` • ${activeGame?.name ?? activeGameId}` : ''}
            </p>
          </div>
        </div>

        <div className="dashboard-entrance__actions">
          <DashboardQuickActions activeGameId={activeGameId} setWorkspaceView={setWorkspaceView} />
        </div>

        <div className="dashboard-entrance__content space-y-6">
          <DashboardIdentitySuggestions
            gameId={activeGameId}
            onOpenSettings={() => {
              setSettingsTab('catalog');
              setWorkspaceView('settings');
            }}
          />
          <DashboardStats stats={stats} />

          {duplicateWasteBytes > 0 && (
            <div
              role="button"
              tabIndex={0}
              className="workspace-interactive alert alert-warning alert-soft alert-horizontal cursor-pointer hover:brightness-95 focus-visible:outline focus-visible:outline-2 focus-visible:outline-warning focus-visible:outline-offset-2"
              onClick={() => setWorkspaceView('storage-optimizer')}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  setWorkspaceView('storage-optimizer');
                }
              }}
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
          />
          <DashboardActivity
            activeGame={activeGame}
            keybindings={keybindings}
            keybindingsLoading={keybindingsLoading}
            recentMods={recentMods}
          />
        </div>
      </div>
    </div>
  );
}
