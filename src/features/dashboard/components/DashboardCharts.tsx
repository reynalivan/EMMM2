import {
  Bar,
  BarChart,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { useTranslation } from 'react-i18next';
import type { DashboardPayload } from '../../../types/dashboard';
import { getChartColors, getChartPalette } from '../dashboardViewUtils';

interface DashboardChartsProps {
  categoryDistribution: DashboardPayload['category_distribution'];
  gameDistribution: DashboardPayload['game_distribution'];
  theme: string;
}

const CHART_TOOLTIP_STYLE = {
  backgroundColor: 'var(--b2)',
  border: '1px solid var(--b3)',
  borderRadius: '1rem',
  fontSize: '0.875rem',
  boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
  backdropFilter: 'blur(8px)',
};

const BAR_GRADIENTS = [
  'url(#gradientPrimary)',
  'url(#gradientSecondary)',
  'url(#gradientAccent)',
  'url(#gradientInfo)',
  'url(#gradientSuccess)',
];

function ChartGradients({ theme }: { theme: string }) {
  const palette = getChartPalette(theme);

  return (
    <defs>
      <linearGradient id="gradientPrimary" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={palette.primary} stopOpacity={0.9} />
        <stop offset="95%" stopColor={palette.primary} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientSecondary" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={palette.secondary} stopOpacity={0.9} />
        <stop offset="95%" stopColor={palette.secondary} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientAccent" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={palette.accent} stopOpacity={0.9} />
        <stop offset="95%" stopColor={palette.accent} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientSuccess" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={palette.success} stopOpacity={0.9} />
        <stop offset="95%" stopColor={palette.success} stopOpacity={0.4} />
      </linearGradient>
      <linearGradient id="gradientInfo" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor={palette.info} stopOpacity={0.9} />
        <stop offset="95%" stopColor={palette.info} stopOpacity={0.4} />
      </linearGradient>
    </defs>
  );
}

export function DashboardCharts({
  categoryDistribution,
  gameDistribution,
  theme,
}: DashboardChartsProps) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <CategoryDistributionChart categoryDistribution={categoryDistribution} theme={theme} />
      <GameDistributionChart gameDistribution={gameDistribution} theme={theme} />
    </div>
  );
}

function CategoryDistributionChart({
  categoryDistribution,
  theme,
}: {
  categoryDistribution: DashboardPayload['category_distribution'];
  theme: string;
}) {
  const { t } = useTranslation(['dashboard']);
  const colors = getChartColors(theme);

  return (
    <div className="card bg-base-200/50 border border-base-300">
      <div className="card-body">
        <h2 className="card-title text-sm font-semibold text-base-content/70">
          {t('charts.category_title')}
        </h2>
        {categoryDistribution.length > 0 ? (
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <ChartGradients theme={theme} />
                <Pie
                  data={categoryDistribution}
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
                  {categoryDistribution.map((_, index) => (
                    <Cell
                      key={`cat-${index}`}
                      fill={colors[index % colors.length]}
                      stroke="var(--b1)"
                      strokeWidth={2}
                    />
                  ))}
                </Pie>
                <Tooltip contentStyle={CHART_TOOLTIP_STYLE} itemStyle={{ color: 'var(--bc)' }} />
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
  );
}

function GameDistributionChart({
  gameDistribution,
  theme,
}: {
  gameDistribution: DashboardPayload['game_distribution'];
  theme: string;
}) {
  const { t } = useTranslation(['dashboard']);

  return (
    <div className="card bg-base-200/50 border border-base-300">
      <div className="card-body">
        <h2 className="card-title text-sm font-semibold text-base-content/70">
          {t('charts.game_title')}
        </h2>
        {gameDistribution.length > 0 ? (
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={gameDistribution} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
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
                  contentStyle={CHART_TOOLTIP_STYLE}
                  itemStyle={{ color: 'var(--bc)' }}
                />
                <Bar
                  dataKey="count"
                  name={t('charts.series_mods')}
                  radius={[8, 8, 0, 0]}
                  animationBegin={200}
                  animationDuration={1500}
                >
                  {gameDistribution.map((_, index) => (
                    <Cell
                      key={`game-${index}`}
                      fill={BAR_GRADIENTS[index % BAR_GRADIENTS.length]}
                    />
                  ))}
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
  );
}
