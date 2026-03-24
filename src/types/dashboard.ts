import type { ModFolder } from './object';

export interface DashboardStats {
  total_mods: number;
  enabled_mods: number;
  disabled_mods: number;
  total_games: number;
  total_size_bytes: number;
  total_collections: number;
}

export interface ChartData {
  name: string;
  value: number;
  color?: string;
}

export interface DashboardPayload {
  stats: DashboardStats;
  duplicate_waste_bytes?: number;
  category_distribution: ChartData[];
  game_distribution: ChartData[];
  recent_mods: ModFolder[];
}
