//! Stable mod/object identity: the deterministic ID policy used by the scanner.

/// Generate a deterministic mod ID from `game_id` + `relative_path`.
/// Uses BLAKE3 hash (first 32 hex chars) so the same folder always gets the same ID.
/// Per TRD §B.6 — replaces random UUID v4 for mod entries.
pub fn generate_stable_id(game_id: &str, folder_path: &str) -> String {
    let key = crate::common::path_key::folder_path_key(folder_path, None);
    let input = format!("{}:{}", game_id, key);
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..32].to_string()
}
