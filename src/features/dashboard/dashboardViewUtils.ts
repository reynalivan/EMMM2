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

export function getChartPalette(theme: string): typeof ONYX_PALETTE {
  return theme === 'onyx' ? ONYX_PALETTE : LIGHT_PALETTE;
}

export function getChartColors(theme: string): string[] {
  return Object.values(getChartPalette(theme));
}
