-- Migration: 20260329133000_object_runtime_projection.sql
-- Purpose: projection-backed runtime counts/status for workspace/object read models

CREATE TABLE IF NOT EXISTS object_runtime_projection (
    game_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_type TEXT,
    mod_count_safe INTEGER NOT NULL DEFAULT 0,
    mod_count_unsafe INTEGER NOT NULL DEFAULT 0,
    enabled_count_safe INTEGER NOT NULL DEFAULT 0,
    enabled_count_unsafe INTEGER NOT NULL DEFAULT 0,
    is_object_disabled INTEGER NOT NULL DEFAULT 0 CHECK(is_object_disabled IN (0, 1)),
    has_naming_conflict INTEGER NOT NULL DEFAULT 0 CHECK(has_naming_conflict IN (0, 1)),
    active_mod_paths_safe_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(active_mod_paths_safe_json)),
    active_mod_paths_unsafe_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(active_mod_paths_unsafe_json)),
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, object_id),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_object
ON object_runtime_projection(game_id, object_id);

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_type
ON object_runtime_projection(game_id, object_type);

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_conflict
ON object_runtime_projection(game_id, has_naming_conflict);

INSERT OR REPLACE INTO object_runtime_projection (
    game_id,
    object_id,
    object_type,
    mod_count_safe,
    mod_count_unsafe,
    enabled_count_safe,
    enabled_count_unsafe,
    is_object_disabled,
    has_naming_conflict,
    active_mod_paths_safe_json,
    active_mod_paths_unsafe_json,
    updated_at
)
SELECT
    o.game_id,
    o.id,
    o.object_type,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_unsafe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_unsafe,
    CASE
        WHEN o.folder_path LIKE 'DISABLED %'
          OR o.folder_path LIKE '%/DISABLED %'
          OR o.folder_path LIKE '%\\DISABLED %'
        THEN 1
        ELSE 0
    END AS is_object_disabled,
    0 AS has_naming_conflict,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_safe_json,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.corridor_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_unsafe_json,
    CURRENT_TIMESTAMP
FROM objects o;
