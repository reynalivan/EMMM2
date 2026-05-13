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

export type DashboardTranslator = (key: string, options?: Record<string, unknown>) => string;

export function getChartColors(theme: string): string[] {
  const palette = theme === 'onyx' ? ONYX_PALETTE : LIGHT_PALETTE;
  return [
    palette.primary,
    palette.secondary,
    palette.accent,
    palette.info,
    palette.success,
    palette.warning,
    palette.error,
    palette.neutral,
  ];
}

export function getChartPalette(theme: string): typeof ONYX_PALETTE {
  return theme === 'onyx' ? ONYX_PALETTE : LIGHT_PALETTE;
}

export function formatRelativeDate(
  dateInput: string | number | null,
  t: DashboardTranslator,
): string {
  if (!dateInput) {
    return t('common:date.unknown');
  }

  const now = Date.now();
  const then = typeof dateInput === 'string' ? new Date(`${dateInput}Z`).getTime() : dateInput;
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);

  if (diffMin < 1) {
    return t('common:date.just_now');
  }

  if (diffMin < 60) {
    return t('common:date.mins_ago', { count: diffMin });
  }

  const diffHrs = Math.floor(diffMin / 60);
  if (diffHrs < 24) {
    return t('common:date.hours_ago', { count: diffHrs });
  }

  const diffDays = Math.floor(diffHrs / 24);
  if (diffDays < 30) {
    return t('common:date.days_ago', { count: diffDays });
  }

  return new Date(then).toLocaleDateString();
}
