//! Stable mod/object identity: the deterministic ID policy used by the scanner.

/// Generate a deterministic mod ID from `game_id` + `relative_path`.
/// Uses BLAKE3 hash (first 32 hex chars) so the same folder always gets the same ID.
/// Per TRD §B.6 — replaces random UUID v4 for mod entries.
pub fn generate_stable_id(game_id: &str, folder_path: &str) -> String {
    let key = crate::common::path_key::folder_path_key(folder_path, None);
    generate_stable_id_from_key(game_id, &key)
}

/// Same policy as [`generate_stable_id`], for callers that already hold the
/// folder path key and must not pay to re-derive it per row.
pub fn generate_stable_id_from_key(game_id: &str, folder_path_key: &str) -> String {
    let hash = blake3::hash(format!("{game_id}:{folder_path_key}").as_bytes());
    hash.to_hex()[..32].to_string()
}
