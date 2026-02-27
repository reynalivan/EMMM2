import {
  Box,
  CircuitBoard,
  Clock,
  Copy,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Keyboard,
  Layers,
  PlayCircle,
  RefreshCw,
  Settings,
} from 'lucide-react';
import {
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/useAppStore';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useDashboardStats } from './hooks/useDashboardStats';
import { useActiveKeybindings } from './hooks/useActiveKeybindings';

// ── Helpers ─────────────────────────────────────────────────────────────────

const CHART_COLORS = [
  '#6366f1', // indigo
  '#f59e0b', // amber
  '#10b981', // emerald
  '#ef4444', // red
  '#8b5cf6', // violet
  '#06b6d4', // cyan
  '#ec4899', // pink
  '#84cc16', // lime
];

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatRelativeDate(dateStr: string | null): string {
  if (!dateStr) return 'Unknown';
  const now = Date.now();
  const then = new Date(dateStr + 'Z').getTime();
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return 'Just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHrs = Math.floor(diffMin / 60);
  if (diffHrs < 24) return `${diffHrs}h ago`;
  const diffDays = Math.floor(diffHrs / 24);
  if (diffDays < 30) return `${diffDays}d ago`;
  return new Date(dateStr).toLocaleDateString();
}

// ── Skeleton Loading ────────────────────────────────────────────────────────

function DashboardSkeleton() {
  return (
    <div className="p-6 space-y-6 animate-pulse">
      {/* Stat tiles skeleton */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="skeleton h-28 rounded-2xl" />
        ))}
      </div>
      {/* Charts skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="skeleton h-72 rounded-2xl" />
        <div className="skeleton h-72 rounded-2xl" />
      </div>
      {/* Activity skeleton */}
      <div className="skeleton h-48 rounded-2xl" />
    </div>
  );
}

// ── Empty State ─────────────────────────────────────────────────────────────

function EmptyState() {
  const { setWorkspaceView } = useAppStore();

  return (
    <div className="flex flex-col items-center justify-center h-full bg-base-100 relative overflow-hidden">
      <div className="absolute inset-0 bg-linear-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none" />
      <div className="z-10 text-center max-w-md px-6">
        <div className="mb-6 relative inline-block">
          <div className="absolute inset-0 bg-primary/20 blur-xl rounded-full" />
          <Gamepad2 size={64} className="text-primary relative z-10 mx-auto" />
        </div>
        <h1 className="text-3xl font-bold mb-3">Welcome to EMMM2</h1>
        <p className="text-base-content/60 mb-8">
          Get started by adding your first game to manage mods.
        </p>
        <button
          onClick={() => setWorkspaceView('settings')}
          className="btn btn-primary btn-lg gap-2"
        >
          <Gamepad2 size={20} />
          Add Your First Game
        </button>
      </div>
    </div>
  );
}

// ── Main Dashboard ──────────────────────────────────────────────────────────

export default function Dashboard() {
  const { data, isLoading, isError, refresh } = useDashboardStats();
  const { keybindings, isLoading: kbLoading } = useActiveKeybindings();
  const { activeGame } = useActiveGame();
  const { setWorkspaceView } = useAppStore();

  // Loading state
  if (isLoading) return <DashboardSkeleton />;

  // Error state
  if (isError || !data) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-4">
        <div role="alert" className="alert alert-error alert-soft max-w-md">
          <CircuitBoard size={20} />
          <span>Failed to load dashboard data.</span>
        </div>
        <button onClick={refresh} className="btn btn-outline btn-sm gap-2">
          <RefreshCw size={16} />
          Retry
        </button>
      </div>
    );
  }

  // Empty state (no games configured)
  if (data.stats.total_games === 0) return <EmptyState />;

  const { stats, duplicate_waste_bytes, category_distribution, game_distribution, recent_mods } =
    data;

  return (
    <div className="h-full overflow-y-auto bg-base-100">
      <div className="p-6 space-y-6 max-w-7xl mx-auto">
        {/* ── Header ─────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
            <p className="text-sm text-base-content/50">
              Overview of your mod ecosystem
              {activeGame ? ` • ${activeGame.name}` : ''}
            </p>
          </div>
          <button
            onClick={refresh}
            className="btn btn-ghost btn-sm gap-2 text-base-content/60 hover:text-base-content"
            aria-label="Refresh dashboard"
          >
            <RefreshCw size={16} />
            Refresh
          </button>
        </div>

        {/* ── Quick Actions Launcher ──────────────────────────────────── */}
        <div className="grid grid-cols-3 sm:grid-cols-5 gap-4">
          <button
            onClick={() => {
              if (activeGame) invoke('launch_game', { gameId: activeGame.id }).catch(console.error);
            }}
            disabled={!activeGame}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-success/10 hover:border-success/30 hover:shadow-lg hover:shadow-success/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed group"
          >
            <div className="w-14 h-14 rounded-2xl bg-success/20 flex items-center justify-center group-hover:bg-success/30 transition-colors">
              <PlayCircle size={28} className="text-success" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-success transition-colors">
              Quick Play
            </span>
          </button>
          <button
            onClick={() => setWorkspaceView('mods')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-primary/10 hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-primary/20 flex items-center justify-center group-hover:bg-primary/30 transition-colors">
              <FolderOpen size={28} className="text-primary" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-primary transition-colors">
              Mods Manager
            </span>
          </button>
          <button
            onClick={() => setWorkspaceView('settings')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-warning/10 hover:border-warning/30 hover:shadow-lg hover:shadow-warning/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-warning/20 flex items-center justify-center group-hover:bg-warning/30 transition-colors">
              <Copy size={28} className="text-warning" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-warning transition-colors">
              Dedup Scanner
            </span>
          </button>
          <button
            onClick={() => setWorkspaceView('collections')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-secondary/10 hover:border-secondary/30 hover:shadow-lg hover:shadow-secondary/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-secondary/20 flex items-center justify-center group-hover:bg-secondary/30 transition-colors">
              <Layers size={28} className="text-secondary" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-secondary transition-colors">
              Collections
            </span>
          </button>
          <button
            onClick={() => setWorkspaceView('settings')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-accent/10 hover:border-accent/30 hover:shadow-lg hover:shadow-accent/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-accent/20 flex items-center justify-center group-hover:bg-accent/30 transition-colors">
              <Settings size={28} className="text-accent" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-accent transition-colors">
              Settings
            </span>
          </button>
        </div>

        {/* ── Overview Tiles ─────────────────────────────────────────── */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <StatTile
            icon={<Box size={22} />}
            title="Total Mods"
            value={stats.total_mods.toLocaleString()}
            desc={`${stats.enabled_mods} enabled · ${stats.disabled_mods} disabled`}
            color="text-primary"
          />
          <StatTile
            icon={<Gamepad2 size={22} />}
            title="Games"
            value={stats.total_games.toLocaleString()}
            desc="Configured"
            color="text-secondary"
          />
          <StatTile
            icon={<HardDrive size={22} />}
            title="Storage"
            value={formatBytes(stats.total_size_bytes)}
            desc="Total mod size"
            color="text-accent"
          />
          <StatTile
            icon={<Layers size={22} />}
            title="Collections"
            value={stats.total_collections.toLocaleString()}
            desc="Presets saved"
            color="text-info"
          />
        </div>

        {/* ── Duplicate Waste Banner ─────────────────────────────────── */}
        {duplicate_waste_bytes > 0 && (
          <div
            role="alert"
            className="alert alert-warning alert-soft alert-horizontal cursor-pointer hover:brightness-95 transition-all"
            onClick={() => setWorkspaceView('settings')}
          >
            <Copy size={20} />
            <div>
              <h3 className="font-bold">Duplicate Waste Detected</h3>
              <p className="text-sm">
                {formatBytes(duplicate_waste_bytes)} wasted on duplicate mods. Run Dedup Scanner to
                clean up.
              </p>
            </div>
          </div>
        )}

        {/* ── Charts Row ─────────────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Category Pie Chart */}
          <div className="card bg-base-200/50 border border-base-300">
            <div className="card-body">
              <h2 className="card-title text-sm font-semibold text-base-content/70">
                Category Distribution
              </h2>
              {category_distribution.length > 0 ? (
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <Pie
                        data={category_distribution}
                        cx="50%"
                        cy="50%"
                        innerRadius={55}
                        outerRadius={85}
                        paddingAngle={3}
                        dataKey="count"
                        nameKey="category"
                        label={({ category, percent }: { category?: string; percent?: number }) =>
                          `${category ?? ''} ${((percent ?? 0) * 100).toFixed(0)}%`
                        }
                      >
                        {category_distribution.map((_, i) => (
                          <Cell key={`cat-${i}`} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                        ))}
                      </Pie>
                      <Tooltip
                        contentStyle={{
                          backgroundColor: 'oklch(var(--b2))',
                          border: '1px solid oklch(var(--b3))',
                          borderRadius: '0.75rem',
                          fontSize: '0.875rem',
                        }}
                      />
                      <Legend />
                    </PieChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <div className="h-64 flex items-center justify-center text-base-content/40">
                  No mod data yet
                </div>
              )}
            </div>
          </div>

          {/* Game Distribution Bar Chart */}
          <div className="card bg-base-200/50 border border-base-300">
            <div className="card-body">
              <h2 className="card-title text-sm font-semibold text-base-content/70">
                Mods per Game
              </h2>
              {game_distribution.length > 0 ? (
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                      data={game_distribution}
                      margin={{ top: 5, right: 10, left: 0, bottom: 5 }}
                    >
                      <XAxis
                        dataKey="game_name"
                        tick={{ fontSize: 12 }}
                        stroke="oklch(var(--bc) / 0.4)"
                      />
                      <YAxis
                        allowDecimals={false}
                        tick={{ fontSize: 12 }}
                        stroke="oklch(var(--bc) / 0.4)"
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: 'oklch(var(--b2))',
                          border: '1px solid oklch(var(--b3))',
                          borderRadius: '0.75rem',
                          fontSize: '0.875rem',
                        }}
                      />
                      <Bar dataKey="count" name="Mods" radius={[6, 6, 0, 0]}>
                        {game_distribution.map((_, i) => (
                          <Cell key={`game-${i}`} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                        ))}
                      </Bar>
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <div className="h-64 flex items-center justify-center text-base-content/40">
                  No game data yet
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Activity Row ───────────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Recently Added */}
          <div className="card bg-base-200/50 border border-base-300 lg:col-span-2">
            <div className="card-body">
              <h2 className="card-title text-sm font-semibold text-base-content/70">
                <Clock size={16} className="mr-1" />
                Recently Added
              </h2>
              {recent_mods.length > 0 ? (
                <ul className="space-y-2">
                  {recent_mods.map((mod) => (
                    <li
                      key={mod.id}
                      className="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-base-300/50 transition-colors"
                    >
                      <div className="min-w-0">
                        <p className="text-sm font-medium truncate">{mod.name}</p>
                        <p className="text-xs text-base-content/50">
                          {mod.game_name}
                          {mod.object_name ? ` · ${mod.object_name}` : ''}
                        </p>
                      </div>
                      <span className="text-xs text-base-content/40 whitespace-nowrap ml-3">
                        {formatRelativeDate(mod.indexed_at)}
                      </span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-sm text-base-content/40 py-4 text-center">No mods indexed yet</p>
              )}
            </div>
          </div>

          {/* Quick Play */}
          <div className="card bg-base-200/50 border border-base-300">
            <div className="card-body items-center text-center">
              <h2 className="card-title text-sm font-semibold text-base-content/70">Quick Play</h2>
              {activeGame ? (
                <>
                  <div className="my-3">
                    <div className="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center">
                      <Gamepad2 size={28} className="text-primary" />
                    </div>
                  </div>
                  <p className="font-semibold text-base">{activeGame.name}</p>
                  <p className="text-xs text-base-content/50 mb-3">Last selected game</p>
                  <button
                    onClick={() => {
                      invoke('launch_game', { gameId: activeGame.id }).catch(console.error);
                    }}
                    className="btn btn-primary btn-sm gap-2 w-full"
                  >
                    <PlayCircle size={16} />
                    Launch
                  </button>
                </>
              ) : (
                <p className="text-sm text-base-content/40 py-4">No game selected</p>
              )}
            </div>
          </div>
        </div>

        {/* ── Active Key Mapping ──────────────────────────────────────── */}
        <div className="card bg-base-200/50 border border-base-300">
          <div className="card-body">
            <h2 className="card-title text-sm font-semibold text-base-content/70">
              <Keyboard size={16} className="mr-1" />
              Active Key Mapping
              {keybindings.length > 0 && (
                <span className="badge badge-sm badge-ghost ml-1">{keybindings.length}</span>
              )}
            </h2>
            {kbLoading ? (
              <div className="flex justify-center py-4">
                <span className="loading loading-dots loading-sm" />
              </div>
            ) : keybindings.length > 0 ? (
              <div className="overflow-x-auto max-h-64">
                <table className="table table-xs table-zebra">
                  <thead className="sticky top-0 bg-base-200">
                    <tr>
                      <th>Mod</th>
                      <th>Section</th>
                      <th>Key</th>
                      <th>Back</th>
                    </tr>
                  </thead>
                  <tbody>
                    {keybindings.map((kb, i) => (
                      <tr key={`${kb.mod_name}-${kb.section_name}-${i}`}>
                        <td className="truncate max-w-[160px]" title={kb.mod_name}>
                          {kb.mod_name}
                        </td>
                        <td className="text-base-content/60">{kb.section_name}</td>
                        <td>{kb.key && <kbd className="kbd kbd-xs">{kb.key}</kbd>}</td>
                        <td>{kb.back && <kbd className="kbd kbd-xs">{kb.back}</kbd>}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="text-sm text-base-content/40 py-4 text-center">
                No keybindings found in enabled mods
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Stat Tile Component ─────────────────────────────────────────────────────

function StatTile({
  icon,
  title,
  value,
  desc,
  color,
}: {
  icon: React.ReactNode;
  title: string;
  value: string;
  desc: string;
  color: string;
}) {
  return (
    <div className="stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300 p-4">
      <div className={`stat-figure ${color}`}>{icon}</div>
      <div className="stat-title text-xs">{title}</div>
      <div className="stat-value text-2xl">{value}</div>
      <div className="stat-desc text-xs">{desc}</div>
    </div>
  );
}
