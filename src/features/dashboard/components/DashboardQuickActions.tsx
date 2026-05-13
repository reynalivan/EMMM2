import { Copy, Download, FolderOpen, Globe, Layers, PlayCircle, Settings } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';

interface DashboardQuickActionsProps {
  activeGameId: string | null;
  setWorkspaceView: (
    view: 'mods' | 'storage-optimizer' | 'collections' | 'settings' | 'browser',
  ) => void;
}

const ACTION_VARIANTS = {
  primary: {
    button: 'hover:bg-primary/10 hover:border-primary/30 hover:shadow-primary/5',
    iconWrapper: 'bg-primary/20 group-hover:bg-primary/30',
    label: 'group-hover:text-primary',
  },
  warning: {
    button: 'hover:bg-warning/10 hover:border-warning/30 hover:shadow-warning/5',
    iconWrapper: 'bg-warning/20 group-hover:bg-warning/30',
    label: 'group-hover:text-warning',
  },
  secondary: {
    button: 'hover:bg-secondary/10 hover:border-secondary/30 hover:shadow-secondary/5',
    iconWrapper: 'bg-secondary/20 group-hover:bg-secondary/30',
    label: 'group-hover:text-secondary',
  },
  accent: {
    button: 'hover:bg-accent/10 hover:border-accent/30 hover:shadow-accent/5',
    iconWrapper: 'bg-accent/20 group-hover:bg-accent/30',
    label: 'group-hover:text-accent',
  },
  info: {
    button: 'hover:bg-info/10 hover:border-info/30 hover:shadow-info/5',
    iconWrapper: 'bg-info/20 group-hover:bg-info/30',
    label: 'group-hover:text-info',
  },
  error: {
    button: 'hover:bg-error/10 hover:border-error/30 hover:shadow-error/5',
    iconWrapper: 'bg-error/20 group-hover:bg-error/30',
    label: 'group-hover:text-error',
  },
} as const;

type ActionVariant = keyof typeof ACTION_VARIANTS;

export function DashboardQuickActions({
  activeGameId,
  setWorkspaceView,
}: DashboardQuickActionsProps) {
  const { t } = useTranslation(['dashboard']);

  return (
    <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-7 gap-4">
      <button
        onClick={() => {
          if (activeGameId) commands.launchGame({ gameId: activeGameId }).catch(console.error);
        }}
        disabled={!activeGameId}
        className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-success/10 hover:border-success/30 hover:shadow-lg hover:shadow-success/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed group"
      >
        <div className="w-14 h-14 rounded-2xl bg-success/20 flex items-center justify-center group-hover:bg-success/30 transition-colors">
          <PlayCircle size={28} className="text-success" />
        </div>
        <span className="text-sm font-semibold text-base-content/80 group-hover:text-success transition-colors">
          {t('actions.quick_play')}
        </span>
      </button>

      <DashboardActionButton
        label={t('actions.mods_manager')}
        colorClass="primary"
        onClick={() => setWorkspaceView('mods')}
        icon={<FolderOpen size={28} className="text-primary" />}
      />
      <DashboardActionButton
        label={t('actions.storage_optimizer')}
        colorClass="warning"
        onClick={() => setWorkspaceView('storage-optimizer')}
        icon={<Copy size={28} className="text-warning" />}
      />
      <DashboardActionButton
        label={t('actions.collections')}
        colorClass="secondary"
        onClick={() => setWorkspaceView('collections')}
        icon={<Layers size={28} className="text-secondary" />}
      />
      <DashboardActionButton
        label={t('actions.settings')}
        colorClass="accent"
        onClick={() => setWorkspaceView('settings')}
        icon={<Settings size={28} className="text-accent" />}
      />
      <DashboardActionButton
        id="dashboard-discover-btn"
        label={t('actions.discover')}
        colorClass="info"
        onClick={() => setWorkspaceView('browser')}
        icon={<Globe size={28} className="text-info" />}
      />
      <DashboardActionButton
        id="dashboard-downloads-btn"
        label={t('actions.downloads')}
        colorClass="error"
        onClick={() => setWorkspaceView('browser')}
        icon={<Download size={28} className="text-error" />}
      />
    </div>
  );
}

function DashboardActionButton({
  id,
  label,
  colorClass,
  icon,
  onClick,
}: {
  id?: string;
  label: string;
  colorClass: ActionVariant;
  icon: React.ReactNode;
  onClick: () => void;
}) {
  const variant = ACTION_VARIANTS[colorClass];

  return (
    <button
      id={id}
      onClick={onClick}
      className={`flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:shadow-lg hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group ${variant.button}`}
    >
      <div
        className={`w-14 h-14 rounded-2xl flex items-center justify-center transition-colors ${variant.iconWrapper}`}
      >
        {icon}
      </div>
      <span
        className={`text-sm font-semibold text-base-content/80 transition-colors ${variant.label}`}
      >
        {label}
      </span>
    </button>
  );
}
