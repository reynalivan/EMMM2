import {
  Box,
  Clock,
  Copy,
  FolderOpen,
  Gamepad2,
  Globe,
  HardDrive,
  Keyboard,
  Layers,
  PlayCircle,
  RefreshCw,
  Settings,
  Download,
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
import { useTranslation } from 'react-i18next';
import type { DashboardPayload } from '../../types/dashboard';
import { useAppStore } from '../../stores/useAppStore';
import { useDashboardStats } from './hooks/useDashboardStats';
import { useActiveKeybindings } from './hooks/useActiveKeybindings';
import { commands } from '../../lib/bindings';
import { useActiveGame } from '../../hooks/useActiveGame';
import { formatBytes } from '../../utils/formatters';

// ── Helpers ─────────────────────────────────────────────────────────────────

const ONYX_PALETTE = {
  primary: '#3b82f6',
  secondary: '#8b5cf6',
  accent: '#b15eff',
  info: '#0ea5e9',
  success: '#10b981',
  warning: '#f59e0b',
  error: '#ef4444',
  neutral: '#111218',
};

const LIGHT_PALETTE = {
  primary: '#5865f2',
  secondary: '#16a34a',
  accent: '#1e293b',
  info: '#0ea5e9',
  success: '#22c55e',
  warning: '#eab308',
  error: '#ef4444',
  neutral: '#cbd5e1',
};

const getChartColors = (theme: string) => {
  const p = theme === 'onyx' ? ONYX_PALETTE : LIGHT_PALETTE;
  return [
    p.primary,
    p.secondary,
    p.accent,
    p.info,
    p.success,
    p.warning,
    p.error,
    p.neutral,
  ];
};

// ── SVG Gradients ───────────────────────────────────────────────────────────

const ChartGradients = ({ theme }: { theme: string }) => {
  const p = theme === 'onyx' ? ONYX_PALETTE : LIGHT_PALETTE;
  return (
    <defs>
      <linearGradient id="gradientPrimary" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={p.primary} stopOpacity={0.9} />
        <stop offset="95%" stopColor={p.primary} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientSecondary" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={p.secondary} stopOpacity={0.9} />
        <stop offset="95%" stopColor={p.secondary} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientAccent" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={p.accent} stopOpacity={0.9} />
        <stop offset="95%" stopColor={p.accent} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientSuccess" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={p.success} stopOpacity={0.9} />
        <stop offset="95%" stopColor={p.success} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientInfo" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={p.info} stopOpacity={0.9} />
        <stop offset="95%" stopColor={p.info} stopOpacity={0.4} />
      </linearGradient>
    </defs>
  );
};

function formatRelativeDate(
  dateInput: string | number | null,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  if (!dateInput) return t('common:date.unknown');
  const now = Date.now();
  const then = typeof dateInput === 'string' ? new Date(dateInput + 'Z').getTime() : dateInput;
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return t('common:date.just_now');
  if (diffMin < 60) return t('common:date.mins_ago', { count: diffMin });
  const diffHrs = Math.floor(diffMin / 60);
  if (diffHrs < 24) return t('common:date.hours_ago', { count: diffHrs });
  const diffDays = Math.floor(diffHrs / 24);
  if (diffDays < 30) return t('common:date.days_ago', { count: diffDays });
  return new Date(then).toLocaleDateString();
}

// ── Skeleton Loading ────────────────────────────────────────────────────────

function DashboardSkeleton() {
  return (
    <div className="p-6 space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-700">
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
  const { t } = useTranslation(['dashboard']);
  const { setWorkspaceView } = useAppStore();

  return (
    <div className="flex flex-col items-center justify-center h-full bg-base-100 relative overflow-hidden">
      <div className="absolute inset-0 bg-linear-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none" />
      <div className="z-10 text-center max-w-md px-6">
        <div className="mb-6 relative inline-block">
          <div className="absolute inset-0 bg-primary/20 blur-xl rounded-full" />
          <Gamepad2 size={64} className="text-primary relative z-10 mx-auto" />
        </div>
        <h1 className="text-3xl font-bold mb-3">{t('empty.title')}</h1>
        <p className="text-base-content/60 mb-8">{t('empty.subtitle')}</p>
        <button
          onClick={() => setWorkspaceView('settings')}
          className="btn btn-primary btn-lg gap-2"
        >
          <Gamepad2 size={20} />
          {t('empty.add_game')}
        </button>
      </div>
    </div>
  );
}

// ── Main Dashboard ──────────────────────────────────────────────────────────

export default function Dashboard() {
  const { t } = useTranslation();
  const setWorkspaceView = useAppStore((s) => s.setWorkspaceView);
  const activeGameId = useAppStore((s) => s.activeGameId);
  const theme = useAppStore((s) => s.theme);
  const colors = getChartColors(theme);

  const { data, isLoading, isError, refresh } = useDashboardStats();
  const { data: activeGame } = useActiveGame();
  const { keybindings, isLoading: kbLoading } = useActiveKeybindings();

  // Loading or Error state
  if (isLoading || isError || !data) {
    return (
      <div className="h-full flex items-center justify-center bg-base-100">
        <span className="loading loading-spinner loading-lg text-primary"></span>
      </div>
    );
  }

  // Empty state (no games configured)
  if (data.stats.total_games === 0) return <EmptyState />;

  const { stats, duplicate_waste_bytes, category_distribution, game_distribution } = data;
  const recent_mods: DashboardPayload['recent_mods'] = data.recent_mods || [];

  return (
    <div className="h-full overflow-y-auto bg-base-100">
      <div className="p-6 space-y-6 max-w-7xl mx-auto">
        {/* ── Header ─────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">{t('header.title')}</h1>
            <p className="text-sm text-base-content/50">
              {t('header.subtitle')}
              {activeGameId ? ` • ${activeGameId}` : ''}
            </p>
          </div>
          <button
            onClick={refresh}
            className="btn btn-ghost btn-sm gap-2 text-base-content/60 hover:text-base-content"
            aria-label={t('header.refresh')}
          >
            <RefreshCw size={16} />
            {t('header.refresh')}
          </button>
        </div>

        {/* ── Quick Actions Launcher ──────────────────────────────────── */}
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
          <button
            onClick={() => setWorkspaceView('mods')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-primary/10 hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-primary/20 flex items-center justify-center group-hover:bg-primary/30 transition-colors">
              <FolderOpen size={28} className="text-primary" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-primary transition-colors">
              {t('actions.mods_manager')}
            </span>
          </button>
          <button
            onClick={() => setWorkspaceView('storage-optimizer')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-warning/10 hover:border-warning/30 hover:shadow-lg hover:shadow-warning/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-warning/20 flex items-center justify-center group-hover:bg-warning/30 transition-colors">
              <Copy size={28} className="text-warning" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-warning transition-colors">
              {t('actions.storage_optimizer')}
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
              {t('actions.collections')}
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
              {t('actions.settings')}
            </span>
          </button>

          {/* Epic 44: Discover Hub */}
          <button
            id="dashboard-discover-btn"
            onClick={() => setWorkspaceView('browser')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-info/10 hover:border-info/30 hover:shadow-lg hover:shadow-info/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group"
          >
            <div className="w-14 h-14 rounded-2xl bg-info/20 flex items-center justify-center group-hover:bg-info/30 transition-colors">
              <Globe size={28} className="text-info" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-info transition-colors">
              {t('actions.discover')}
            </span>
          </button>

          {/* Epic 44: Download Manager */}
          <button
            id="dashboard-downloads-btn"
            onClick={() => setWorkspaceView('browser')}
            className="flex flex-col items-center gap-3 py-6 px-4 rounded-2xl bg-base-200/60 border border-base-300 hover:bg-error/10 hover:border-error/30 hover:shadow-lg hover:shadow-error/5 hover:scale-[1.04] active:scale-[0.96] transition-all cursor-pointer group relative"
          >
            <div className="w-14 h-14 rounded-2xl bg-error/20 flex items-center justify-center group-hover:bg-error/30 transition-colors">
              <Download size={28} className="text-error" />
            </div>
            <span className="text-sm font-semibold text-base-content/80 group-hover:text-error transition-colors">
              {t('actions.downloads')}
            </span>
          </button>
        </div>

        {/* ── Overview Tiles ─────────────────────────────────────────── */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <StatTile
            icon={<Box size={22} />}
            title={t('stats.total_mods')}
            value={stats.total_mods.toLocaleString()}
            desc={`${stats.enabled_mods} ${t('stats.enabled')} · ${stats.disabled_mods} ${t('stats.disabled')}`}
            color="text-primary"
            bgGradient="from-primary/10 to-transparent"
          />
          <StatTile
            icon={<Gamepad2 size={22} />}
            title={t('stats.games')}
            value={stats.total_games.toLocaleString()}
            desc={t('stats.configured')}
            color="text-secondary"
            bgGradient="from-secondary/10 to-transparent"
          />
          <StatTile
            icon={<HardDrive size={22} />}
            title={t('stats.storage')}
            value={formatBytes(stats.total_size_bytes)}
            desc={t('stats.total_size_desc')}
            color="text-accent"
            bgGradient="from-accent/10 to-transparent"
          />
          <StatTile
            icon={<Layers size={22} />}
            title={t('stats.collections')}
            value={stats.total_collections.toLocaleString()}
            desc={t('stats.presets_desc')}
            color="text-info"
            bgGradient="from-info/10 to-transparent"
          />
        </div>

        {/* ── Duplicate Waste Banner ─────────────────────────────────── */}
        {typeof duplicate_waste_bytes !== 'undefined' && duplicate_waste_bytes > 0 && (
          <div
            role="alert"
            className="alert alert-warning alert-soft alert-horizontal cursor-pointer hover:brightness-95 transition-all"
            onClick={() => setWorkspaceView('storage-optimizer')}
          >
            <Copy size={20} />
            <div>
              <h3 className="font-bold">{t('waste.title')}</h3>
              <p className="text-sm">
                {t('waste.subtitle', { size: formatBytes(duplicate_waste_bytes) })}
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
                {t('charts.category_title')}
              </h2>
              {category_distribution.length > 0 ? (
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart>
                      <ChartGradients theme={theme} />
                      <Pie
                        data={category_distribution}
                        cx="50%"
                        cy="50%"
                        innerRadius={55}
                        outerRadius={85}
                        paddingAngle={4}
                        dataKey="count"
                        nameKey="category"
                        animationBegin={0}
                        animationDuration={1200}
                        label={({ category, percent }: { category?: string; percent?: number }) =>
                          `${category ?? ''} ${((percent ?? 0) * 100).toFixed(0)}%`
                        }
                      >
                        {category_distribution.map((_, i: number) => (
                          <Cell
                            key={`cat-${i}`}
                            fill={colors[i % colors.length]}
                            stroke="var(--b1)"
                            strokeWidth={2}
                          />
                        ))}
                      </Pie>
                      <Tooltip
                        contentStyle={{
                          backgroundColor: 'var(--b2)',
                          border: '1px solid var(--b3)',
                          borderRadius: '1rem',
                          fontSize: '0.875rem',
                          boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
                          backdropFilter: 'blur(8px)',
                        }}
                        itemStyle={{ color: 'var(--bc)' }}
                      />
                      <Legend verticalAlign="bottom" height={36} />
                    </PieChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <div className="h-64 flex items-center justify-center text-base-content/40">
                  {t('charts.no_mod_data')}
                </div>
              )}
            </div>
          </div>

          {/* Game Distribution Bar Chart */}
          <div className="card bg-base-200/50 border border-base-300">
            <div className="card-body">
              <h2 className="card-title text-sm font-semibold text-base-content/70">
                {t('charts.game_title')}
              </h2>
              {game_distribution.length > 0 ? (
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                      data={game_distribution}
                      margin={{ top: 5, right: 10, left: 0, bottom: 5 }}
                    >
                      <ChartGradients theme={theme} />
                      <XAxis
                        dataKey="game_name"
                        tick={{ fontSize: 11, fill: 'var(--bc)', opacity: 0.6 }}
                        axisLine={false}
                        tickLine={false}
                      />
                      <YAxis
                        allowDecimals={false}
                        tick={{ fontSize: 11, fill: 'var(--bc)', opacity: 0.6 }}
                        axisLine={false}
                        tickLine={false}
                      />
                      <Tooltip
                        cursor={{ fill: 'var(--bc)', fillOpacity: 0.05 }}
                        contentStyle={{
                          backgroundColor: 'var(--b2)',
                          border: '1px solid var(--b3)',
                          borderRadius: '1rem',
                          fontSize: '0.875rem',
                          boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
                          backdropFilter: 'blur(8px)',
                        }}
                        itemStyle={{ color: 'var(--bc)' }}
                      />
                      <Bar
                        dataKey="count"
                        name="Mods"
                        radius={[8, 8, 0, 0]}
                        animationBegin={200}
                        animationDuration={1500}
                      >
                        {game_distribution.map((_, i: number) => {
                          const gradients = [
                            'url(#gradientPrimary)',
                            'url(#gradientSecondary)',
                            'url(#gradientAccent)',
                            'url(#gradientInfo)',
                            'url(#gradientSuccess)',
                          ];
                          return (
                            <Cell key={`game-${i}`} fill={gradients[i % gradients.length]} />
                          );
                        })}
                      </Bar>
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <div className="h-64 flex items-center justify-center text-base-content/40">
                  {t('charts.no_game_data')}
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
                {t('activity.recent_title')}
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
                          {mod.category
                            ? t('activity.category', { category: mod.category })
                            : t('activity.uncategorized')}
                        </p>
                      </div>
                      <span className="text-xs text-base-content/40 whitespace-nowrap ml-3">
                        {formatRelativeDate(mod.modified_at, t)}
                      </span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="text-sm text-base-content/40 py-4 text-center">
                  {t('activity.no_mods')}
                </p>
              )}
            </div>
          </div>

          {/* Quick Play */}
          <div className="card bg-base-200/50 border border-base-300">
            <div className="card-body items-center text-center">
              <h2 className="card-title text-sm font-semibold text-base-content/70">
                {t('actions.quick_play')}
              </h2>
              {activeGameId ? (
                <>
                  <div className="my-3">
                    <div className="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center">
                      <Gamepad2 size={28} className="text-primary" />
                    </div>
                  </div>
                  <p className="font-semibold text-base">{activeGame.name}</p>
                  <p className="text-xs text-base-content/50 mb-3">{t('activity.last_selected')}</p>
                  <button
                    onClick={() => {
                      commands.launchGame({ gameId: activeGame.id }).catch(console.error);
                    }}
                    className="btn btn-primary btn-sm gap-2 w-full"
                  >
                    <PlayCircle size={16} />
                    {t('activity.launch')}
                  </button>
                </>
              ) : (
                <p className="text-sm text-base-content/40 py-4">{t('activity.no_game')}</p>
              )}
            </div>
          </div>
        </div>

        {/* ── Active Key Mapping ──────────────────────────────────────── */}
        <div className="card bg-base-200/50 border border-base-300">
          <div className="card-body">
            <h2 className="card-title text-sm font-semibold text-base-content/70">
              <Keyboard size={16} className="mr-1" />
              {t('keys.title')}
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
                      <th>{t('keys.table_mod')}</th>
                      <th>{t('keys.table_section')}</th>
                      <th>{t('keys.table_key')}</th>
                      <th>{t('keys.table_back')}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {keybindings.map((kb, i) => (
                      <tr key={`${kb.mod_name}-${kb.section_name}-${i}`}>
                        <td
                          className="truncate max-w-40"
                          title={(kb.mod_name as string) ?? undefined}
                        >
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
                {t('keys.no_bindings')}
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
  bgGradient,
}: {
  icon: React.ReactNode;
  title: string;
  value: string;
  desc: string;
  color: string;
  bgGradient: string;
}) {
  return (
    <div className={`stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300 p-4 relative overflow-hidden group hover:bg-base-300/40 transition-colors`}>
      <div className={`absolute inset-0 bg-linear-to-br ${bgGradient} opacity-0 group-hover:opacity-100 transition-opacity`} />
      <div className={`stat-figure ${color} relative z-10 scale-100 group-hover:scale-110 transition-transform`}>
        {icon}
      </div>
      <div className="stat-title text-xs relative z-10">{title}</div>
      <div className="stat-value text-2xl relative z-10 group-hover:translate-x-1 transition-transform">
        {value}
      </div>
      <div className="stat-desc text-xs relative z-10">{desc}</div>
    </div>
  );
}
