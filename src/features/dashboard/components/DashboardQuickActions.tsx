import { Copy, Download, FolderOpen, Globe, Layers, PlayCircle, Settings } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import type { WorkspaceView } from '../../../stores/appStore/navigationSlice';

interface DashboardQuickActionsProps {
  activeGameId: string | null;
  setWorkspaceView: (view: WorkspaceView) => void;
}

export function DashboardQuickActions({
  activeGameId,
  setWorkspaceView,
}: DashboardQuickActionsProps) {
  const { t } = useTranslation(['dashboard']);

  return (
    <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-7 gap-3">
      {/* Launching the game is the only action that earns colour. The rest are
          navigation, and navigation that shouts in warning-orange or error-red
          reads as an alert about a problem that isn't there. */}
      <ActionTile
        label={t('actions.quick_play')}
        icon={<PlayCircle size={26} />}
        onClick={() => {
          if (activeGameId) commands.launchGame(activeGameId).catch(console.error);
        }}
        disabled={!activeGameId}
        emphasis
      />
      <ActionTile
        label={t('actions.mods_manager')}
        icon={<FolderOpen size={26} />}
        onClick={() => setWorkspaceView('mods')}
      />
      <ActionTile
        label={t('actions.storage_optimizer')}
        icon={<Copy size={26} />}
        onClick={() => setWorkspaceView('storage-optimizer')}
      />
      <ActionTile
        label={t('actions.collections')}
        icon={<Layers size={26} />}
        onClick={() => setWorkspaceView('collections')}
      />
      <ActionTile
        label={t('actions.settings')}
        icon={<Settings size={26} />}
        onClick={() => setWorkspaceView('settings')}
      />
      <ActionTile
        id="dashboard-discover-btn"
        label={t('actions.discover')}
        icon={<Globe size={26} />}
        onClick={() => setWorkspaceView('browser')}
      />
      <ActionTile
        id="dashboard-downloads-btn"
        label={t('actions.downloads')}
        icon={<Download size={26} />}
        onClick={() => setWorkspaceView('downloads')}
      />
    </div>
  );
}

function ActionTile({
  id,
  label,
  icon,
  onClick,
  disabled,
  emphasis,
}: {
  id?: string;
  label: string;
  icon: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  emphasis?: boolean;
}) {
  return (
    <button
      id={id}
      onClick={onClick}
      disabled={disabled}
      className={`group flex flex-col items-center gap-2.5 py-5 px-3 rounded-2xl border bg-base-200/60 transition-[background-color,border-color,transform] duration-150 cursor-pointer active:scale-[0.98] disabled:opacity-40 disabled:cursor-not-allowed disabled:active:scale-100 ${
        emphasis
          ? 'border-primary/30 bg-primary/10 hover:bg-primary/15 hover:border-primary/50'
          : 'border-base-300 hover:bg-base-300/60 hover:border-base-content/15'
      }`}
    >
      <span
        className={`w-12 h-12 rounded-xl flex items-center justify-center transition-colors duration-150 ${
          emphasis
            ? 'bg-primary/20 text-primary group-hover:bg-primary/30'
            : 'bg-base-content/5 text-base-content/60 group-hover:bg-base-content/10 group-hover:text-base-content'
        }`}
      >
        {icon}
      </span>
      <span
        className={`text-xs font-medium text-center leading-tight transition-colors duration-150 ${
          emphasis ? 'text-primary' : 'text-base-content/70 group-hover:text-base-content'
        }`}
      >
        {label}
      </span>
    </button>
  );
}
