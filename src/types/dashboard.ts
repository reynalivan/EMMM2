// ── Epic 13: Dashboard Types ──────────────────────────────────────────────────

/** Global overview statistics */
export interface DashboardStats {
  total_mods: number;
  enabled_mods: number;
  disabled_mods: number;
  total_size_bytes: number;
  total_games: number;
  total_collections: number;
}

/** Pie chart slice for category distribution */
export interface CategorySlice {
  category: string;
  count: number;
}

/** Bar chart data for game distribution */
export interface GameSlice {
  game_id: string;
  game_name: string;
  count: number;
}

/** Recently indexed mod entry */
export interface RecentMod {
  id: string;
  name: string;
  game_name: string;
  object_name: string | null;
  indexed_at: string | null;
}

/** Combined payload from get_dashboard_stats command */
export interface DashboardPayload {
  stats: DashboardStats;
  duplicate_waste_bytes: number;
  category_distribution: CategorySlice[];
  game_distribution: GameSlice[];
  recent_mods: RecentMod[];
}

/** A keybinding extracted from an enabled mod's INI file */
export interface ActiveKeyBinding {
  mod_name: string;
  section_name: string;
  key: string | null;
  back: string | null;
}
