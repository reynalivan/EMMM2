import {
  Copy,
  Download,
  FolderOpen,
  Globe,
  Inbox,
  Layers,
  PlayCircle,
  Settings,
} from 'lucide-react';
import type { ReactNode } from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceView } from '@/app/store';
import { useAppStore } from '@/app/store';
import { launchConfiguredGame } from '@/entities/game';
import { formatAppError } from '@/shared/lib/appError';
import { toast } from '@/shared/ui/toast';

interface DashboardQuickActionsProps {
  activeGameId: string | null;
  setWorkspaceView: (view: WorkspaceView) => void;
}

export function DashboardQuickActions({
  activeGameId,
  setWorkspaceView,
}: DashboardQuickActionsProps) {
  const { t } = useTranslation(['dashboard']);
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const [isLaunching, setIsLaunching] = useState(false);

  const handleQuickPlay = async () => {
    if (!activeGameId) return;
    setIsLaunching(true);
    try {
      await launchConfiguredGame(activeGameId, autoCloseLauncher);
    } catch (cause) {
      toast.error(formatAppError(cause));
    } finally {
      setIsLaunching(false);
    }
  };

  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-4 lg:grid-cols-8">
      {/* Launching the game is the only action that earns colour. The rest are
          navigation, and navigation that shouts in warning-orange or error-red
          reads as an alert about a problem that isn't there. */}
      <ActionTile
        label={t('actions.quick_play')}
        icon={
          isLaunching ? (
            <span className="loading loading-spinner loading-md" />
          ) : (
            <PlayCircle size={26} />
          )
        }
        onClick={() => void handleQuickPlay()}
        disabled={!activeGameId || isLaunching}
        emphasis
      />
      <ActionTile
        label={t('actions.mods_manager')}
        icon={<FolderOpen size={26} />}
        onClick={() => setWorkspaceView('mods')}
      />
      <ActionTile
        label={t('actions.mod_inbox')}
        icon={<Inbox size={26} />}
        onClick={() => setWorkspaceView('mod-inbox')}
      />
      <ActionTile
        label={t('actions.collections')}
        icon={<Layers size={26} />}
        onClick={() => setWorkspaceView('collections')}
      />
      <ActionTile
        label={t('actions.storage_optimizer')}
        icon={<Copy size={26} />}
        onClick={() => setWorkspaceView('storage-optimizer')}
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
      <ActionTile
        label={t('actions.settings')}
        icon={<Settings size={26} />}
        onClick={() => setWorkspaceView('settings')}
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
  const button = (
    <button
      id={id}
      onClick={onClick}
      disabled={disabled}
      className={`group flex h-full w-full cursor-pointer items-center gap-3 rounded-2xl border px-4 py-3 transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40 sm:flex-col sm:gap-2.5 sm:px-3 sm:py-5 ${
        emphasis
          ? 'border-base-300 bg-base-200/60 hover:border-base-content/15 hover:bg-base-300/60'
          : 'border-base-300 bg-base-200/60 hover:bg-base-300/60 hover:border-base-content/15'
      }`}
    >
      <span
        className={`w-12 h-12 rounded-xl flex items-center justify-center transition-colors duration-150 ${
          emphasis
            ? 'bg-primary/15 text-primary group-hover:bg-primary/20'
            : 'bg-base-content/5 text-base-content/60 group-hover:bg-base-content/10 group-hover:text-base-content'
        }`}
      >
        {icon}
      </span>
      <span
        className={`whitespace-nowrap text-xs font-medium leading-tight transition-colors duration-150 sm:text-center ${
          emphasis
            ? 'text-base-content/80 group-hover:text-base-content'
            : 'text-base-content/70 group-hover:text-base-content'
        }`}
      >
        {label}
      </span>
    </button>
  );

  return button;
}
