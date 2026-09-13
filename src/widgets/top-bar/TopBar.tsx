import { useState, useRef, useEffect } from 'react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ChevronLeft,
  Copy,
  FolderOpen,
  Inbox,
  LayoutGrid,
  Layers,
  PlayCircle,
  Settings,
} from 'lucide-react';
import { useAppStore } from '@/app/store';
import { launchConfiguredGame, useActiveGame } from '@/entities/game';
import { formatAppError } from '@/shared/lib/appError';
import { toast } from '@/shared/ui/toast';
import GameSelector from './GameSelector';
import GlobalActions from './GlobalActions';
import { SafetyFilterControl } from '@/shared/ui/components/ui/SafetyFilterControl';
import { LiquidSurface } from '@/shared/ui/liquid';

export interface TopBarProps {
  launchBar?: ReactNode;
  contextControls?: ReactNode;
}

export default function TopBar({ launchBar, contextControls }: TopBarProps) {
  const { t } = useTranslation('layout');
  const workspaceView = useAppStore((state) => state.workspaceView);
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const setSafetyFilter = useAppStore((state) => state.setSafetyFilter);
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const { activeGame } = useActiveGame();
  const [menuOpen, setMenuOpen] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
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
                ? t('nav.browser')
                : workspaceView === 'downloads'
                  ? t('nav.downloads')
                  : t('nav.storage_optimizer');
  const showLaunchBar = workspaceView === 'dashboard' || workspaceView === 'mods';

  const handleQuickPlay = async () => {
    if (!activeGame) return;
    setIsLaunching(true);
    try {
      await launchConfiguredGame(activeGame.id, autoCloseLauncher);
      setMenuOpen(false);
    } catch (cause) {
      toast.error(formatAppError(cause));
    } finally {
      setIsLaunching(false);
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
      id: 'settings' as const,
      icon: Settings,
      label: t('nav.settings'),
    },
    {
      id: 'storage-optimizer' as const,
      icon: Copy,
      label: t('nav.storage_optimizer'),
    },
  ];

  // Close on click outside
  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [menuOpen]);

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
            onClick={() => setMenuOpen((v) => !v)}
            className={`cursor-pointer rounded-lg border p-2 transition-[background-color,border-color,color] duration-150 ${
              menuOpen
                ? 'border-primary/30 bg-primary/10 text-primary'
                : 'border-base-300 bg-base-200 text-base-content/75 hover:bg-base-300 hover:text-base-content'
            }`}
            title={t('nav.app_menu_tip')}
          >
            <LayoutGrid size={20} />
          </button>

          {/* Dropdown Menu */}
          {menuOpen && (
            <LiquidSurface
              liquidRole="overlay"
              data-testid="app-menu-overlay"
              className="app-menu-overlay absolute left-0 top-full z-[var(--workspace-layer-popover)] mt-2 w-56 rounded-xl shadow-lg"
              contentClassName="p-2"
            >
              {workspaceView !== 'dashboard' && (
                <>
                  <button
                    type="button"
                    data-testid="nav-dashboard"
                    onClick={() => {
                      setWorkspaceView('dashboard');
                      setMenuOpen(false);
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
                      setMenuOpen(false);
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
            </LiquidSurface>
          )}
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
              <GameSelector compact />
            </div>
            <div className="hidden sm:block">
              <GameSelector />
            </div>
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
    </LiquidSurface>
  );
}
