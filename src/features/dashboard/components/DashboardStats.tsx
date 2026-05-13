import { Box, Gamepad2, HardDrive, Layers } from 'lucide-react';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { DashboardStats as DashboardStatsPayload } from '../../../types/dashboard';
import { formatBytes } from '../../../utils/formatters';

interface DashboardStatsProps {
  stats: DashboardStatsPayload;
}

export function DashboardStats({ stats }: DashboardStatsProps) {
  const { t } = useTranslation(['dashboard']);

  return (
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
  );
}

function StatTile({
  icon,
  title,
  value,
  desc,
  color,
  bgGradient,
}: {
  icon: ReactNode;
  title: string;
  value: string;
  desc: string;
  color: string;
  bgGradient: string;
}) {
  return (
    <div className="stat bg-base-200/50 backdrop-blur rounded-2xl border border-base-300 p-4 relative overflow-hidden group hover:bg-base-300/40 transition-colors">
      <div
        className={`absolute inset-0 bg-linear-to-br ${bgGradient} opacity-0 group-hover:opacity-100 transition-opacity`}
      />
      <div
        className={`stat-figure ${color} relative z-10 scale-100 group-hover:scale-110 transition-transform`}
      >
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
