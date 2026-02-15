import { ArrowRight, Box, CircuitBoard, Layers } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';

export default function Dashboard() {
  const { setWorkspaceView, activeGame } = useAppStore();

  return (
    <div className="flex flex-col items-center justify-center h-full bg-base-100 relative overflow-hidden">
      {/* Background Decor */}
      <div className="absolute inset-0 bg-linear-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none" />
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-primary/10 rounded-full blur-3xl pointer-events-none opacity-50 animate-pulse" />

      <div className="z-10 text-center max-w-2xl px-6">
        <div className="mb-8 relative inline-block">
          <div className="absolute inset-0 bg-primary/20 blur-xl rounded-full" />
          <CircuitBoard size={64} className="text-primary relative z-10 mx-auto" />
        </div>

        <h1 className="text-5xl font-bold mb-4 tracking-tight bg-linear-to-r from-base-content to-base-content/60 bg-clip-text text-transparent">
          Welcome to EMMM2
        </h1>
        <p className="text-xl text-base-content/60 mb-12">
          The next-generation mod manager for
          <span className="font-semibold text-primary ml-1">
            {activeGame === 'GIMI'
              ? 'Genshin Impact'
              : activeGame === 'SRMI'
                ? 'Star Rail'
                : 'Zenless Zone Zero'}
          </span>
        </p>

        <div className="grid grid-cols-3 gap-4 mb-12 w-full max-w-lg mx-auto">
          <div className="stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300">
            <div className="stat-figure text-primary">
              <Box size={24} />
            </div>
            <div className="stat-title">Mods</div>
            <div className="stat-value text-2xl">1,204</div>
            <div className="stat-desc">Indexed</div>
          </div>
          <div className="stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300">
            <div className="stat-figure text-secondary">
              <Layers size={24} />
            </div>
            <div className="stat-title">Presets</div>
            <div className="stat-value text-2xl">8</div>
            <div className="stat-desc">Active</div>
          </div>
          <div className="stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300">
            <div className="stat-figure text-accent">
              <CircuitBoard size={24} />
            </div>
            <div className="stat-title">Plugins</div>
            <div className="stat-value text-2xl">3</div>
            <div className="stat-desc">Loaded</div>
          </div>
        </div>

        <button
          onClick={() => setWorkspaceView('mods')}
          className="btn btn-primary btn-lg gap-3 shadow-xl hover:shadow-primary/20 hover:scale-105 transition-all w-64 group"
        >
          Open Mod Manager
          <ArrowRight className="group-hover:translate-x-1 transition-transform" />
        </button>
      </div>

      <div className="absolute bottom-6 text-sm text-base-content/30">
        v0.1.0-alpha • Early Access
      </div>
    </div>
  );
}
