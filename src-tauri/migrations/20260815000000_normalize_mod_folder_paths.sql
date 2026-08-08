-- Normalize `mods.folder_path` to the mods-root-relative form.
--
-- Two writers disagreed: disk reconcile stored a path relative to the mods
-- root, the scan commit stored an absolute one. Readers had to guess, and
-- conflict detection guessed wrong -- it tested the column with a plain
-- existence check, which fails for a relative path and silently reported no
-- conflicts. The scan commit now writes the relative form; this brings rows
-- written before that in line.
--
-- Deliberately NOT touched:
--   * folder_path_key -- computed as root.join(path), so it is already the
--     same value for either form. Rewriting it would be a no-op at best.
--   * id -- derived from that key, and referenced by four tables
--     (collections, mod_versions, dedup whitelist pairs, and one SET NULL).
--     Nothing here needs the id to change.
--
-- Conservative by construction: a row is rewritten only when its path starts
-- with the game's mods root, case-insensitively, and the next character is a
-- separator. A trailing separator on mods_path, a mixed separator style, or a
-- path outside the root all fail that test and the row is left exactly as it
-- was. Skipping is always safe here; mangling a path is not.
UPDATE mods
SET folder_path = substr(mods.folder_path, length(g.mods_path) + 2)
FROM games g
WHERE g.id = mods.game_id
  AND g.mods_path IS NOT NULL
  AND g.mods_path <> ''
  AND length(mods.folder_path) > length(g.mods_path) + 1
  AND lower(substr(mods.folder_path, 1, length(g.mods_path))) = lower(g.mods_path)
  AND substr(mods.folder_path, length(g.mods_path) + 1, 1) IN ('/', '\');
