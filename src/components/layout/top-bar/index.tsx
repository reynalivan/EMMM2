import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ChevronLeft,
  Copy,
  FolderOpen,
  LayoutGrid,
  Layers,
  PlayCircle,
  Settings,
} from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { commands } from '../../../lib/bindings';
import GameSelector from './GameSelector';
import ContextControls from './ContextControls';
import GlobalActions from './GlobalActions';

export default function TopBar() {
  const { t } = useTranslation('layout');
  const { workspaceView, setWorkspaceView } = useAppStore();
  const { activeGame } = useActiveGame();
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const NAV_ITEMS = [
    {
      id: 'dashboard' as const,
      icon: LayoutGrid,
      label: t('nav.dashboard'),
      color: 'text-primary',
      bg: 'bg-primary/15',
    },
    {
      id: 'mods' as const,
      icon: FolderOpen,
      label: t('nav.mods_manager'),
      color: 'text-info',
      bg: 'bg-info/15',
    },
    {
      id: 'collections' as const,
      icon: Layers,
      label: t('nav.collections'),
      color: 'text-secondary',
      bg: 'bg-secondary/15',
    },
    {
      id: 'settings' as const,
      icon: Settings,
      label: t('nav.settings'),
      color: 'text-accent',
      bg: 'bg-accent/15',
    },
    {
      id: 'storage-optimizer' as const,
      icon: Copy,
      label: t('nav.storage_optimizer'),
      color: 'text-warning',
      bg: 'bg-warning/15',
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
    <div className="h-16 glass-surface flex items-center justify-between px-3 md:px-4 sticky top-0 z-50">
      {/* Left Section: Branding & Navigation */}
      <div className="flex items-center gap-3 md:gap-5 shrink-0">
        {/* App Menu Toggle */}
        <div className="relative" ref={menuRef}>
          <button
            onClick={() => setMenuOpen((v) => !v)}
            className={`p-2 rounded-xl border transition-all cursor-pointer ${
              menuOpen
                ? 'bg-primary/20 text-primary border-primary/30 shadow-[0_0_15px_-5px_var(--color-primary)]'
                : 'bg-primary/10 text-primary border-primary/20 hover:bg-primary/15 shadow-[0_0_15px_-5px_var(--color-primary)]'
            }`}
            title={t('nav.app_menu_tip')}
          >
            <LayoutGrid size={20} />
          </button>

          {/* Dropdown Menu */}
          {menuOpen && (
            <div className="absolute top-full left-0 mt-2 w-56 bg-base-200 border border-base-300 rounded-2xl shadow-2xl p-2 z-60 animate-in fade-in slide-in-from-top-2 duration-150">
              {/* Quick Play */}
              <button
                onClick={() => {
                  if (activeGame)
                    commands.launchGame({ gameId: activeGame.id }).catch(console.error);
                  setMenuOpen(false);
                }}
                disabled={!activeGame}
                className="flex items-center gap-3 w-full px-3 py-2.5 rounded-xl text-left hover:bg-success/10 transition-colors disabled:opacity-40 disabled:cursor-not-allowed group"
              >
                <div className="w-8 h-8 rounded-lg bg-success/15 flex items-center justify-center shrink-0">
                  <PlayCircle size={16} className="text-success" />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-base-content/90 group-hover:text-success transition-colors">
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
                return (
                  <button
                    key={item.id}
                    onClick={() => {
                      setWorkspaceView(item.id);
                      setMenuOpen(false);
                    }}
                    className={`flex items-center gap-3 w-full px-3 py-2.5 rounded-xl text-left transition-colors group ${
                      isActive ? 'bg-primary/10' : 'hover:bg-base-300/60'
                    }`}
                  >
                    <div
                      className={`w-8 h-8 rounded-lg ${item.bg} flex items-center justify-center shrink-0`}
                    >
                      <Icon size={16} className={item.color} />
                    </div>
                    <span
                      className={`text-sm font-medium ${isActive ? 'text-primary' : 'text-base-content/80 group-hover:text-base-content'} transition-colors`}
                    >
                      {item.label}
                    </span>
                    {isActive && <div className="ml-auto w-1.5 h-1.5 rounded-full bg-primary" />}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* Back button when not on dashboard */}
        {workspaceView !== 'dashboard' && (
          <button
            onClick={() => setWorkspaceView('dashboard')}
            className="btn btn-ghost btn-sm btn-square text-base-content/70 hover:text-primary hover:bg-base-content/5"
            title={t('nav.back_to_dashboard')}
          >
            <ChevronLeft size={20} />
          </button>
        )}

        <div className="flex flex-col gap-0.5">
          <span className="font-bold text-lg md:text-xl leading-none tracking-tight text-base-content glow-text">
            {t('app.name')}
          </span>
          <span className="text-[9px] md:text-[10px] uppercase tracking-[0.2em] opacity-70 font-semibold hidden sm:inline-block text-accent">
            {t('app.version')}
          </span>
        </div>

        <div className="h-8 w-px bg-base-content/5 mx-1 md:mx-2 hidden xs:block" />

        {/* Game Selector Cartridge */}
        <GameSelector />
      </div>

      {/* Center Section: Context Controls (Desktop Only) */}
      <ContextControls />

      {/* Right Section: Global Actions */}
      <GlobalActions />
    </div>
  );
}
