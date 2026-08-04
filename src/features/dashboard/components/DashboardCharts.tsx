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

interface DashboardChartsProps {
  categoryDistribution: DashboardPayload['category_distribution'];
  gameDistribution: DashboardPayload['game_distribution'];
}

/**
 * Charts read the live daisyUI theme tokens. The previous hard-coded JS palettes
 * were a third copy of the colours (after the two @plugin blocks in App.css) and
 * had already drifted out of sync with them, which is how light and dark ended up
 * with different semantic hues. `neutral` is deliberately not in the series — it
 * is a near-black brand neutral in both themes and vanished against the card.
 */
const CHART_SERIES = [
  'var(--color-primary)',
  'var(--color-secondary)',
  'var(--color-accent)',
  'var(--color-info)',
  'var(--color-success)',
  'var(--color-warning)',
  'var(--color-error)',
];

const CHART_TOOLTIP_STYLE = {
  backgroundColor: 'var(--color-base-200)',
  border: '1px solid var(--color-base-300)',
  borderRadius: '1rem',
  fontSize: '0.875rem',
  boxShadow: '0 10px 15px -3px rgb(0 0 0 / 0.25)',
  backdropFilter: 'blur(8px)',
};

const GRADIENT_SERIES = ['primary', 'secondary', 'accent', 'info', 'success'] as const;

const BAR_GRADIENTS = GRADIENT_SERIES.map((name) => `url(#gradient-${name})`);

function ChartGradients() {
  return (
    <defs>
      {GRADIENT_SERIES.map((name) => (
        <linearGradient key={name} id={`gradient-${name}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="5%" stopColor={`var(--color-${name})`} stopOpacity={0.9} />
          <stop offset="95%" stopColor={`var(--color-${name})`} stopOpacity={0.4} />
        </linearGradient>
      ))}
    </defs>
  );
}

export function DashboardCharts({ categoryDistribution, gameDistribution }: DashboardChartsProps) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <CategoryDistributionChart categoryDistribution={categoryDistribution} />
      <GameDistributionChart gameDistribution={gameDistribution} />
    </div>
  );
}

function CategoryDistributionChart({
  categoryDistribution,
}: {
  categoryDistribution: DashboardPayload['category_distribution'];
}) {
  const { t } = useTranslation(['dashboard']);

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
                <ChartGradients />
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
                      fill={CHART_SERIES[index % CHART_SERIES.length]}
                      stroke="var(--color-base-100)"
                      strokeWidth={2}
                    />
                  ))}
                </Pie>
                <Tooltip
                  contentStyle={CHART_TOOLTIP_STYLE}
                  itemStyle={{ color: 'var(--color-base-content)' }}
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
  );
}

function GameDistributionChart({
  gameDistribution,
}: {
  gameDistribution: DashboardPayload['game_distribution'];
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
                <ChartGradients />
                <XAxis
                  dataKey="game_name"
                  tick={{ fontSize: 11, fill: 'var(--color-base-content)', opacity: 0.6 }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  allowDecimals={false}
                  tick={{ fontSize: 11, fill: 'var(--color-base-content)', opacity: 0.6 }}
                  axisLine={false}
                  tickLine={false}
                />
                <Tooltip
                  cursor={{ fill: 'var(--color-base-content)', fillOpacity: 0.05 }}
                  contentStyle={CHART_TOOLTIP_STYLE}
                  itemStyle={{ color: 'var(--color-base-content)' }}
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
