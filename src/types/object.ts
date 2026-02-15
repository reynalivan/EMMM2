/**
 * Types for Epic 3: Game Object Management.
 * Matches SQLite `objects` table schema from migration 004.
 */

/** Full game object record from DB */
export interface GameObject {
  id: string;
  game_id: string;
  name: string;
  object_type: string;
  sub_category: string | null;
  sort_order: number;
  tags: string; // JSON string of string array
  metadata: string; // JSON string of metadata object
  thumbnail_path: string | null;
  is_safe: boolean;
  created_at: string;
}

/** Lightweight summary for sidebar rendering (joined with mod counts) */
export interface ObjectSummary {
  id: string;
  name: string;
  object_type: string;
  sub_category: string | null;
  mod_count: number;
  enabled_count: number;
  thumbnail_path: string | null;
  is_safe: boolean;
}

/** Filter criteria sent to DB queries */
export interface ObjectFilter {
  game_id: string;
  search_query?: string;
  object_type?: string;
  safe_mode: boolean;
  meta_filters?: Record<string, string[]>;
}

/** Game schema: defines categories and filter fields per game type */
export interface GameSchema {
  categories: CategoryDef[];
  filters: FilterDef[];
}

export interface CategoryDef {
  name: string;
  icon: string;
  color: string;
}

export interface FilterDef {
  key: string;
  label: string;
  options: string[];
}

/** Category count for sidebar badges */
export interface CategoryCount {
  object_type: string;
  count: number;
}

/** Input DTO for creating an object */
export interface CreateObjectInput {
  game_id: string;
  name: string;
  object_type: string;
  sub_category?: string;
  tags?: string[];
  metadata?: Record<string, unknown>;
}

/** Input DTO for updating an object */
export interface UpdateObjectInput {
  name?: string;
  object_type?: string;
  sub_category?: string;
  tags?: string[];
  metadata?: Record<string, unknown>;
  thumbnail_path?: string;
  is_safe?: boolean;
}
