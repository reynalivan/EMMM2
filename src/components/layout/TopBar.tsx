import { ChevronLeft, LayoutGrid } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import GameSelector from './TopBar/GameSelector';
import ContextControls from './TopBar/ContextControls';
import GlobalActions from './TopBar/GlobalActions';

export default function TopBar() {
  const { workspaceView, setWorkspaceView } = useAppStore();

  return (
    <div className="h-16 glass-surface flex items-center justify-between px-3 md:px-4 sticky top-0 z-50">
      {/* Left Section: Branding & Navigation */}
      <div className="flex items-center gap-3 md:gap-5 shrink-0">
        {workspaceView !== 'dashboard' ? (
          <button
            onClick={() => setWorkspaceView('dashboard')}
            className="btn btn-ghost btn-sm btn-square text-white/70 hover:text-primary hover:bg-white/5"
            title="Back to Dashboard"
          >
            <ChevronLeft size={20} />
          </button>
        ) : (
          <div className="p-2 bg-primary/10 rounded-xl text-primary border border-primary/20 shadow-[0_0_15px_-5px_var(--color-primary)]">
            <LayoutGrid size={20} />
          </div>
        )}

        <div className="flex flex-col">
          <span className="font-bold text-lg md:text-xl leading-none tracking-tight text-white glow-text">
            EMMM2
          </span>
          <span className="text-[9px] md:text-[10px] uppercase tracking-[0.2em] opacity-50 font-semibold hidden sm:inline-block text-accent">
            Mod Manager
          </span>
        </div>

        <div className="h-8 w-px bg-white/5 mx-1 md:mx-2 hidden xs:block" />

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
