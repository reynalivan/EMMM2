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
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
      <StatTile
        icon={<Box size={18} />}
        title={t('stats.total_mods')}
        value={stats.total_mods.toLocaleString()}
        desc={`${stats.enabled_mods} ${t('stats.enabled')} · ${stats.disabled_mods} ${t('stats.disabled')}`}
      />
      <StatTile
        icon={<Gamepad2 size={18} />}
        title={t('stats.games')}
        value={stats.total_games.toLocaleString()}
        desc={t('stats.configured')}
      />
      <StatTile
        icon={<HardDrive size={18} />}
        title={t('stats.storage')}
        value={formatBytes(stats.total_size_bytes)}
        desc={t('stats.total_size_desc')}
      />
      <StatTile
        icon={<Layers size={18} />}
        title={t('stats.collections')}
        value={stats.total_collections.toLocaleString()}
        desc={t('stats.presets_desc')}
      />
    </div>
  );
}

/* Read-only figures: no hover lift, no per-tile hue. The number is the loudest
   thing on the tile, which is the whole point of the tile. */
function StatTile({
  icon,
  title,
  value,
  desc,
}: {
  icon: ReactNode;
  title: string;
  value: string;
  desc: string;
}) {
  return (
    <div className="rounded-2xl border border-base-300 bg-base-200/50 p-4">
      <div className="flex items-center gap-2 text-base-content/50">
        {icon}
        <span className="text-xs font-medium uppercase tracking-wide truncate">{title}</span>
      </div>
      <div className="mt-2 text-2xl font-semibold tabular-nums leading-tight">{value}</div>
      <div className="mt-0.5 text-xs text-base-content/50 truncate" title={desc}>
        {desc}
      </div>
    </div>
  );
}
