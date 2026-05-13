export interface DashboardStats {
  total_mods: number;
  enabled_mods: number;
  disabled_mods: number;
  total_games: number;
  total_size_bytes: number;
  total_collections: number;
}

export interface CategorySlice {
  category: string;
  count: number;
}

export interface GameSlice {
  game_id: string;
  game_name: string;
  count: number;
}

export interface RecentMod {
  id: string;
  name: string;
  game_name: string;
  object_name: string | null;
  indexed_at: string | null;
}

export interface DashboardPayload {
  stats: DashboardStats;
  duplicate_waste_bytes?: number;
  category_distribution: CategorySlice[];
  game_distribution: GameSlice[];
  recent_mods: RecentMod[];
}
