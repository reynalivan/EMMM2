import { createPortal } from 'react-dom';
import { useState, useRef, useEffect, useLayoutEffect, useCallback } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  AlertTriangle,
  ChevronLeft,
  Copy,
  Download,
  FolderOpen,
  Globe,
  Inbox,
  LayoutGrid,
  Layers,
  LoaderCircle,
  PlayCircle,
  RotateCcw,
  Settings,
} from 'lucide-react';
import { useAppStore } from '@/app/store';
import { launchConfiguredGame, useActiveGame } from '@/entities/game';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { toast } from '@/shared/ui/toast';
import { useBackgroundIndexingStatus } from '@/pages/onboarding/hooks/useBackgroundIndexingStatus';
import GameSelector from './GameSelector';
import GlobalActions from './GlobalActions';
import { SafetyFilterControl } from '@/shared/ui/components/ui/SafetyFilterControl';
import { LiquidSurface } from '@/shared/ui/liquid';

const APP_MENU_WIDTH_PX = 224;
const APP_MENU_GAP_PX = 8;

interface AppMenuPosition {
  left: number;
  top: number;
}

export interface TopBarProps {
  launchBar?: ReactNode;
  contextControls?: ReactNode;
}

function OnboardingIndexingIndicator({
  sessions,
}: Pick<
  ReturnType<typeof useBackgroundIndexingStatus>,
  'sessions'
>) {
  const { t } = useTranslation('layout');
  const session = sessions.find((candidate) =>
    candidate.games.some((game) => game.phase !== 'Ready'),
  );

  if (!session) return null;

  const needsAttention = session.games.some(
    (game) => game.phase === 'NeedsAttention' || game.phase === 'Failed',
  );

  const label = needsAttention
    ? t('game_selector.indexing.background_attention')
    : t('game_selector.indexing.background', {
        completed: session.completed_games,
        total: session.total_games,
      });
  return (
    <div
      role="status"
      aria-live="polite"
      title={label}
      className="hidden items-center gap-1.5 rounded-md border border-base-content/15 bg-base-content/5 px-2 py-1 text-[11px] font-medium text-base-content/65 md:flex"
    >
      {needsAttention ? (
        <AlertTriangle size={13} className="text-warning" aria-hidden="true" />
      ) : (
        <LoaderCircle
          size={13}
          className="animate-spin motion-reduce:animate-none"
          aria-hidden="true"
        />
      )}
      <span>{label}</span>
    </div>
  );
}

export default function TopBar({ launchBar, contextControls }: TopBarProps) {
  const { t } = useTranslation(['layout', 'common']);
  const workspaceView = useAppStore((state) => state.workspaceView);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const isAppMenuOpen = useAppStore((state) => state.isAppMenuOpen);
  const setAppMenuOpen = useAppStore((state) => state.setAppMenuOpen);
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const setSafetyFilter = useAppStore((state) => state.setSafetyFilter);
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const { activeGame } = useActiveGame();
  const backgroundIndexingStatus = useBackgroundIndexingStatus();
  const runtimeSync = useAppStore((state) =>
    activeGame?.id ? state.runtimeSyncByGame?.[activeGame.id] : undefined,
  );
  const gameActivation = useAppStore((state) =>
    activeGame?.id ? state.gameActivationByGame?.[activeGame.id] : undefined,
  );
  const [menuOpen, setMenuOpen] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);
  const [isRetryingRuntime, setIsRetryingRuntime] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const menuOverlayRef = useRef<HTMLDivElement>(null);
  const wasAppMenuOpen = useRef(isAppMenuOpen);
  const [menuPosition, setMenuPosition] = useState<AppMenuPosition | null>(null);
  const pageTitle =
    workspaceView === 'dashboard'
      ? t('nav.dashboard')
      : workspaceView === 'mods'
        ? t('nav.mods_manager')
        : workspaceView === 'mod-inbox'
          ? t('nav.mod_inbox')
          : workspaceView === 'collections'
            ? t('nav.collections')
            : workspaceView === 'settings'
              ? t('nav.settings')
              : workspaceView === 'browser'
                ? t('nav.discover')
                : workspaceView === 'downloads'
                  ? t('nav.downloads')
                  : t('nav.storage_optimizer');
  const showLaunchBar = workspaceView === 'dashboard' || workspaceView === 'mods';

  const closeAppMenu = useCallback(() => {
    setMenuOpen(false);
    setMenuPosition(null);
    setAppMenuOpen(false);
  }, [setAppMenuOpen]);

  useEffect(() => {
    if (wasAppMenuOpen.current && !isAppMenuOpen) {
      setMenuOpen(false);
      setMenuPosition(null);
    }
    wasAppMenuOpen.current = isAppMenuOpen;
  }, [isAppMenuOpen]);

  useEffect(() => () => setAppMenuOpen(false), [setAppMenuOpen]);

  const handleQuickPlay = async () => {
    if (!activeGame) return;
    setIsLaunching(true);
    try {
      await launchConfiguredGame(activeGame.id, autoCloseLauncher);
      closeAppMenu();
    } catch (cause) {
      toast.error(formatAppError(cause));
    } finally {
      setIsLaunching(false);
    }
  };

  const handleRuntimeRetry = async () => {
    if (!activeGame || isRetryingRuntime || (gameActivation && gameActivation.phase !== 'ready')) {
      return;
    }
    setIsRetryingRuntime(true);
    try {
      await commands.retryRuntimeSync(activeGame.id);
    } catch (cause) {
      toast.error(formatAppError(cause));
    } finally {
      setIsRetryingRuntime(false);
    }
  };

  const NAV_ITEMS = [
    {
      id: 'dashboard' as const,
      icon: LayoutGrid,
      label: t('nav.dashboard'),
    },
    {
      id: 'mods' as const,
      icon: FolderOpen,
      label: t('nav.mods_manager'),
    },
    {
      id: 'mod-inbox' as const,
      icon: Inbox,
      label: t('nav.mod_inbox'),
    },
    {
      id: 'collections' as const,
      icon: Layers,
      label: t('nav.collections'),
    },
    {
      id: 'storage-optimizer' as const,
      icon: Copy,
      label: t('nav.storage_optimizer'),
    },
    {
      id: 'browser' as const,
      icon: Globe,
      label: t('nav.discover'),
    },
    {
      id: 'downloads' as const,
      icon: Download,
      label: t('nav.downloads'),
    },
    {
      id: 'settings' as const,
      icon: Settings,
      label: t('nav.settings'),
    },
  ];

  const handleMenuToggle = () => {
    if (menuOpen) {
      closeAppMenu();
      return;
    }

    setMenuOpen(true);
    setAppMenuOpen(true);
  };

  useLayoutEffect(() => {
    if (!menuOpen) return;

    const updateMenuPosition = () => {
      const button = menuButtonRef.current;
      if (!button) return;

      const rect = button.getBoundingClientRect();
      const maxLeft = Math.max(
        APP_MENU_GAP_PX,
        window.innerWidth - APP_MENU_WIDTH_PX - APP_MENU_GAP_PX,
      );

      setMenuPosition({
        left: Math.min(Math.max(rect.left, APP_MENU_GAP_PX), maxLeft),
        top: rect.bottom + APP_MENU_GAP_PX,
      });
    };

    updateMenuPosition();
    window.addEventListener('resize', updateMenuPosition);
    return () => window.removeEventListener('resize', updateMenuPosition);
  }, [menuOpen]);

  // Close on click outside
  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      const clickedInsideMenu =
        menuRef.current?.contains(target) || menuOverlayRef.current?.contains(target);

      if (!clickedInsideMenu) {
        closeAppMenu();
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [closeAppMenu, menuOpen]);

  const appMenuContent = (
    <>
      {workspaceView !== 'dashboard' && (
        <>
          <button
            type="button"
            data-testid="nav-dashboard"
            onClick={() => {
              setWorkspaceView('dashboard');
              closeAppMenu();
            }}
            className="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm font-medium text-base-content/85 transition-colors hover:bg-base-content/8 hover:text-base-content"
          >
            <ChevronLeft size={16} className="text-base-content/60" />
            {t('nav.back_to_dashboard')}
          </button>
          <div className="mx-2 my-1.5 h-px bg-base-content/10" />
        </>
      )}

      {/* Quick Play */}
      <button
        onClick={() => void handleQuickPlay()}
        disabled={!activeGame || isLaunching}
        className="flex items-center gap-3 w-full px-3 py-2.5 rounded-xl text-left hover:bg-success/10 transition-colors disabled:opacity-40 disabled:cursor-not-allowed group"
      >
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/10">
          <PlayCircle size={16} className="text-primary" />
        </div>
        <div className="min-w-0">
          <p className="text-sm font-medium text-base-content/90 transition-colors group-hover:text-primary">
            {t('nav.quick_play')}
          </p>
          <p className="text-[10px] text-base-content/40 truncate">
            {activeGame ? activeGame.name : t('nav.no_game_selected')}
          </p>
        </div>
      </button>

      <div className="h-px bg-base-300 my-1.5 mx-2" />

      {/* Navigation Items */}
      {NAV_ITEMS.map((item) => {
        const Icon = item.icon;
        const isActive = workspaceView === item.id;
        if (item.id === 'dashboard' && workspaceView !== 'dashboard') return null;

        return (
          <button
            key={item.id}
            data-testid={`nav-${item.id}`}
            onClick={() => {
              setWorkspaceView(item.id);
              closeAppMenu();
            }}
            className={`flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors group ${
              isActive ? 'bg-base-content/5' : 'hover:bg-base-300/60'
            }`}
          >
            <div
              className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-lg ${
                isActive ? 'text-base-content' : 'text-base-content/55'
              }`}
            >
              <Icon size={16} />
            </div>
            <span
              className={`text-sm font-medium ${isActive ? 'text-base-content' : 'text-base-content/80 group-hover:text-base-content'} transition-colors`}
            >
              {item.label}
            </span>
          </button>
        );
      })}
    </>
  );

  const appMenuOverlay =
    menuOpen && menuPosition && typeof document !== 'undefined'
      ? createPortal(
          <div
            ref={menuOverlayRef}
            className="fixed z-[var(--workspace-layer-popover)] w-56 overflow-y-auto"
            style={{
              left: menuPosition.left,
              top: menuPosition.top,
              maxHeight: `calc(100dvh - ${menuPosition.top + APP_MENU_GAP_PX}px)`,
            }}
          >
            <LiquidSurface
              liquidRole="overlay"
              data-testid="app-menu-overlay"
              className="app-menu-overlay w-full rounded-xl shadow-lg"
              contentClassName="p-2"
            >
              {appMenuContent}
            </LiquidSurface>
          </div>,
          document.body,
        )
      : null;

  return (
    <LiquidSurface
      liquidRole="nav"
      className="absolute inset-x-0 top-0 z-[var(--workspace-layer-topbar)] h-16"
      contentClassName="flex h-full items-center gap-2 px-3 md:px-4"
    >
      {/* Left Section: Branding & Navigation */}
      <div className="flex shrink-0 items-center gap-2 sm:gap-3 md:gap-4">
        {/* App Menu Toggle */}
        <div className="relative" ref={menuRef}>
          <button
            ref={menuButtonRef}
            onClick={handleMenuToggle}
            className={`cursor-pointer rounded-lg border p-2 transition-[background-color,border-color,color] duration-150 ${
              menuOpen
                ? 'border-primary/30 bg-primary/10 text-primary'
                : 'border-base-300 bg-base-200 text-base-content/75 hover:bg-base-300 hover:text-base-content'
            }`}
            title={t('nav.app_menu_tip')}
          >
            <LayoutGrid size={20} />
          </button>
        </div>

        {workspaceView === 'settings' ? (
          <div className="hidden flex-col gap-0.5 sm:flex">
            <span className="font-bold text-lg leading-none tracking-tight text-base-content">
              {t('app.name')}
            </span>
            <span className="hidden text-[9px] font-semibold uppercase tracking-[0.2em] text-base-content/50 sm:inline-block md:text-[10px]">
              {t('app.version')}
            </span>
          </div>
        ) : (
          <>
            <div className="sm:hidden">
              <GameSelector compact backgroundIndexingStatus={backgroundIndexingStatus} />
            </div>
            <div className="hidden sm:block">
              <GameSelector backgroundIndexingStatus={backgroundIndexingStatus} />
            </div>
            <OnboardingIndexingIndicator sessions={backgroundIndexingStatus.sessions} />
            {runtimeSync &&
              (runtimeSync.phase === 'queued' ||
                runtimeSync.phase === 'running' ||
                runtimeSync.phase === 'needs_manual_reload' ||
                runtimeSync.phase === 'failed') && (
                <div
                  role="status"
                  aria-live="polite"
                  title={
                    runtimeSync.message ??
                    (runtimeSync.phase === 'failed'
                      ? t('common:reconcile.runtime_sync_failed')
                      : runtimeSync.phase === 'needs_manual_reload'
                        ? t('common:reconcile.manual_reload_short')
                        : t('common:reconcile.runtime_syncing'))
                  }
                  className={`hidden items-center gap-1.5 rounded-md border px-2 py-1 text-[11px] font-medium md:flex ${
                    runtimeSync.phase === 'failed' || runtimeSync.phase === 'needs_manual_reload'
                      ? 'border-warning/30 bg-warning/10 text-warning'
                      : 'border-base-content/15 bg-base-content/5 text-base-content/65'
                  }`}
                >
                  {runtimeSync.phase === 'queued' || runtimeSync.phase === 'running' ? (
                    <LoaderCircle size={13} className="animate-spin" aria-hidden="true" />
                  ) : (
                    <AlertTriangle size={13} aria-hidden="true" />
                  )}
                  <span
                    className={runtimeSync.phase === 'failed' ? 'max-w-64 truncate' : undefined}
                  >
                    {runtimeSync.phase === 'failed'
                      ? (runtimeSync.message ?? t('common:reconcile.runtime_sync_failed'))
                      : runtimeSync.phase === 'needs_manual_reload'
                        ? t('common:reconcile.manual_reload_short')
                        : t('common:reconcile.runtime_syncing')}
                  </span>
                  {runtimeSync.phase === 'failed' &&
                    (!gameActivation || gameActivation.phase === 'ready') && (
                      <button
                        type="button"
                        className="btn btn-ghost btn-xs h-5 min-h-0 gap-1 px-1"
                        onClick={() => void handleRuntimeRetry()}
                        disabled={isRetryingRuntime}
                        aria-label={t('common:actions.retry')}
                      >
                        <RotateCcw
                          size={11}
                          className={isRetryingRuntime ? 'animate-spin' : undefined}
                          aria-hidden="true"
                        />
                        {t('common:actions.retry')}
                      </button>
                    )}
                </div>
              )}
          </>
        )}
      </div>

      {/* Center Section: Active Page Title or Mods Manager collection switcher */}
      <div
        data-testid="topbar-center"
        className={`flex min-w-0 flex-1 items-center justify-center px-1 pointer-events-none ${
          workspaceView === 'mods' && contextControls
            ? 'xl:absolute xl:left-1/2 xl:-translate-x-1/2 xl:flex-none xl:px-0'
            : ''
        }`}
      >
        {workspaceView === 'mods' && contextControls ? (
          <>
            <span className="max-w-36 truncate text-[10px] font-bold uppercase tracking-[0.12em] text-muted sm:max-w-none sm:text-xs sm:tracking-[0.16em] lg:text-sm lg:tracking-[0.2em] xl:hidden">
              {pageTitle}
            </span>
            <div className="pointer-events-auto hidden min-w-0 xl:block">{contextControls}</div>
          </>
        ) : (
          <span className="max-w-36 truncate text-[10px] font-bold uppercase tracking-[0.12em] text-muted sm:max-w-none sm:text-xs sm:tracking-[0.16em] lg:text-sm lg:tracking-[0.2em]">
            {pageTitle}
          </span>
        )}
      </div>

      {/* Right Section: Context Actions */}
      <div
        className="flex min-w-0 shrink-0 items-center justify-end gap-1 sm:min-w-[100px] sm:gap-2 xl:ml-auto"
        id="topbar-actions-portal"
      >
        {workspaceView === 'mods' && (
          <div className="hidden items-center xl:flex">
            <SafetyFilterControl value={safetyFilter} onChange={setSafetyFilter} compact />
          </div>
        )}
        {workspaceView !== 'mod-inbox' && (
          <>
            <GlobalActions
              launchBar={showLaunchBar ? launchBar : undefined}
              safetyFilter={workspaceView === 'mods' ? safetyFilter : undefined}
              onSafetyFilterChange={workspaceView === 'mods' ? setSafetyFilter : undefined}
            />
          </>
        )}
      </div>
      {appMenuOverlay}
    </LiquidSurface>
  );
}
