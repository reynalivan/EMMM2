/**
 * Epic 3: Object Service — frontend-side DB queries via @tauri-apps/plugin-sql.
 * Follows existing pattern from `src/lib/db.ts`.
 *
 * All CRUD queries for the `objects` table run from the frontend,
 * matching the project's architecture (Rust handles migrations only).
 */

import { select, execute } from '../lib/db';
import type {
  ObjectSummary,
  ObjectFilter,
  CategoryCount,
  UpdateObjectInput,
  CreateObjectInput,
} from '../types/object';

/**
 * Reserved Windows filenames that cannot be used as object names.
 * Covers: NC-3.3-03 (Invalid Name)
 */
const RESERVED_NAMES = new Set([
  'CON',
  'PRN',
  'AUX',
  'NUL',
  'COM1',
  'COM2',
  'COM3',
  'COM4',
  'COM5',
  'COM6',
  'COM7',
  'COM8',
  'COM9',
  'LPT1',
  'LPT2',
  'LPT3',
  'LPT4',
  'LPT5',
  'LPT6',
  'LPT7',
  'LPT8',
  'LPT9',
]);

/**
 * Validate an object name for filesystem safety.
 * Returns error message if invalid, null if valid.
 */
export function validateObjectName(name: string): string | null {
  const trimmed = name.trim();

  if (!trimmed || trimmed.length < 2) {
    return 'Name must be at least 2 characters.';
  }

  if (trimmed.length > 255) {
    return 'Name must be at most 255 characters.';
  }

  if (RESERVED_NAMES.has(trimmed.toUpperCase())) {
    return `"${trimmed}" is a reserved system name.`;
  }

  if (/[<>:"/\\|?*]/.test(trimmed)) {
    return 'Name contains invalid characters: < > : " / \\ | ? *';
  }

  if (/^\.+$/.test(trimmed)) {
    return 'Name cannot be only dots.';
  }

  if (trimmed.includes('..')) {
    return 'Name cannot contain path traversal (dot-dot).';
  }

  return null;
}

/**
 * Fetch objects for a game, with optional filtering.
 * Returns lightweight summaries with mod counts.
 * Covers: TC-3.1-01, TC-3.1-02, EC-3.05
 */
export async function getObjects(filter: ObjectFilter): Promise<ObjectSummary[]> {
  let query = `
    SELECT 
      o.id,
      o.name,
      o.object_type,
      o.sub_category,
      o.thumbnail_path,
      o.is_safe,
      o.is_pinned,
      o.is_auto_sync,
      o.created_at,
      o.metadata,
      o.tags,
      COUNT(m.id) as mod_count,
      COUNT(CASE WHEN m.status = 'ENABLED' THEN 1 END) as enabled_count
    FROM objects o
    LEFT JOIN mods m ON m.object_id = o.id
    WHERE o.game_id = $1
  `;
  const params: unknown[] = [filter.game_id];
  let paramIdx = 2;

  // Safe Mode filter
  if (filter.safe_mode) {
    query += ` AND o.is_safe = 1`;
  }

  // Object type filter
  if (filter.object_type) {
    query += ` AND o.object_type = $${paramIdx}`;
    params.push(filter.object_type);
    paramIdx++;
  }

  // Search query (name + tags)
  if (filter.search_query && filter.search_query.trim()) {
    const searchTerm = `%${filter.search_query.trim().toLowerCase()}%`;
    query += ` AND (LOWER(o.name) LIKE $${paramIdx} OR LOWER(o.tags) LIKE $${paramIdx})`;
    params.push(searchTerm);
    paramIdx++;
  }

  // Meta filters (Dynamic JSON query)
  if (filter.meta_filters && Object.keys(filter.meta_filters).length > 0) {
    for (const [key, values] of Object.entries(filter.meta_filters)) {
      if (values && values.length > 0) {
        // Construct IN clause manually for JSON extract
        // Security Note: 'key' comes from schema (trusted), 'values' are from UI
        // We use parameters for values to prevent injection
        const placeholders = values.map(() => `$${paramIdx++}`).join(', ');
        // JSON_EXTRACT returns the value as string if it's a string in JSON
        query += ` AND JSON_EXTRACT(o.metadata, '$.${key}') IN (${placeholders})`;
        params.push(...values);
      }
    }
  }

  query += ` GROUP BY o.id`;

  // Status Filter (HAVING clause)
  if (filter.status_filter === 'enabled') {
    query += ` HAVING enabled_count > 0`;
  } else if (filter.status_filter === 'disabled') {
    // Show objects that have mods but none are enabled
    query += ` HAVING mod_count > 0 AND enabled_count = 0`;
  }

  // Sorting
  switch (filter.sort_by) {
    case 'date':
      query += ` ORDER BY o.is_pinned DESC, o.created_at DESC`;
      break;
    case 'rarity':
      // Sort by rarity (descending, so 5-Star first)
      query += ` ORDER BY o.is_pinned DESC, JSON_EXTRACT(o.metadata, '$.rarity') DESC, o.name ASC`;
      break;
    case 'name':
    default:
      query += ` ORDER BY o.is_pinned DESC, o.object_type, o.name ASC`;
      break;
  }

  return select<ObjectSummary[]>(query, params);
}

/**
 * Get category counts for sidebar badges.
 * Covers: TC-3.1-02
 */
export async function getCategoryCounts(
  gameId: string,
  safeMode: boolean,
): Promise<CategoryCount[]> {
  let query = `
    SELECT object_type, COUNT(*) as count
    FROM objects
    WHERE game_id = $1
  `;
  const params: unknown[] = [gameId];

  if (safeMode) {
    query += ` AND is_safe = 1`;
  }

  query += ` GROUP BY object_type ORDER BY object_type`;

  return select<CategoryCount[]>(query, params);
}

/**
 * Create a new object (US-3.3).
 * Covers: TC-3.3-01, NC-3.3-01 (Duplicate)
 */
export async function createObject(input: CreateObjectInput): Promise<string> {
  const nameError = validateObjectName(input.name);
  if (nameError) throw new Error(nameError);

  const id = crypto.randomUUID();
  try {
    await execute(
      `INSERT INTO objects (id, game_id, name, object_type, sub_category, is_safe, is_auto_sync, tags, metadata, created_at)
       VALUES ($1, $2, $3, $4, $5, $6, 0, '[]', $7, datetime('now'))`,
      [
        id,
        input.game_id,
        input.name.trim(),
        input.object_type,
        input.sub_category ?? null,
        input.is_safe !== false ? 1 : 0,
        input.metadata ? JSON.stringify(input.metadata) : '{}',
      ],
    );
  } catch (err) {
    const msg = String(err).toLowerCase();
    if (msg.includes('unique constraint failed') || msg.includes('unique_game_object_name')) {
      // eslint-disable-next-line preserve-caught-error
      throw new Error(`An object named "${input.name.trim()}" already exists for this game.`);
    }
    throw err;
  }
  return id;
}

/**
 * Update an existing object.
 * Covers: TC-3.3-02
 */
export async function updateObject(id: string, updates: UpdateObjectInput): Promise<void> {
  const setClauses: string[] = [];
  const params: unknown[] = [];
  let paramIdx = 1;

  if (updates.name !== undefined) {
    const nameError = validateObjectName(updates.name);
    if (nameError) throw new Error(nameError);
    setClauses.push(`name = $${paramIdx++}`);
    params.push(updates.name.trim());
  }

  if (updates.object_type !== undefined) {
    setClauses.push(`object_type = $${paramIdx++}`);
    params.push(updates.object_type);
  }

  if (updates.sub_category !== undefined) {
    setClauses.push(`sub_category = $${paramIdx++}`);
    params.push(updates.sub_category);
  }

  if (updates.tags !== undefined) {
    setClauses.push(`tags = $${paramIdx++}`);
    params.push(JSON.stringify(updates.tags));
  }

  if (updates.metadata !== undefined) {
    setClauses.push(`metadata = $${paramIdx++}`);
    params.push(JSON.stringify(updates.metadata));
  }

  if (updates.thumbnail_path !== undefined) {
    setClauses.push(`thumbnail_path = $${paramIdx++}`);
    params.push(updates.thumbnail_path);
  }

  if (updates.is_safe !== undefined) {
    setClauses.push(`is_safe = $${paramIdx++}`);
    params.push(updates.is_safe ? 1 : 0);
  }

  if (updates.is_auto_sync !== undefined) {
    setClauses.push(`is_auto_sync = $${paramIdx++}`);
    params.push(updates.is_auto_sync ? 1 : 0);
  }

  if (setClauses.length === 0) return;

  params.push(id);
  const query = `UPDATE objects SET ${setClauses.join(', ')} WHERE id = $${paramIdx}`;
  await execute(query, params);
}

/**
 * Delete an object by ID.
 * Covers: NC-3.3-02
 */
export async function deleteObject(id: string): Promise<void> {
  await execute('DELETE FROM objects WHERE id = $1', [id]);
}
