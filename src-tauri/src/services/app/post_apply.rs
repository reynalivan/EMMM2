use crate::repo;
use crate::services::config::AppSettings;
use crate::services::corridor_service;
use crate::services::keyviewer::generator;
use crate::services::keyviewer::harvester;
use crate::services::keyviewer::matcher;
use crate::services::keyviewer::resource_pack;
use crate::services::mods::metadata;
use crate::services::scanner::watcher::WatcherSuppressor;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Context for post-mutation tasks.
#[derive(Clone)]
pub struct PostApplyContext {
    pub pool: SqlitePool,
    pub game_id: String,
    pub is_safe: bool,
    pub mods_path: PathBuf,
    pub suppressor: Arc<WatcherSuppressor>,
    pub settings: AppSettings,
    /// Optional status overrides (e.g. preset name, folder name) from the mutation source.
    pub status_fields: Option<generator::StatusFields>,
}

/// Run tasks that should execute after any mod state change (Toggle, Apply, Switch).
///
/// Tasks include:
/// 1. Recomputing corridor signature (DB)
/// 2. Harvesting hashes from enabled mods
/// 3. Matching characters & generating KeyViewer.ini + keybind texts
/// 4. Refreshing conflict cache
/// 5. Updating runtime status banner
pub async fn run_post_apply_tasks(ctx: PostApplyContext) -> Result<(), String> {
    let pool = &ctx.pool;
    let game_id = &ctx.game_id;
    let is_safe = ctx.is_safe;
    let mods_path = &ctx.mods_path;

    log::info!(
        "[post_apply] Starting post-apply tasks for game={}",
        game_id
    );

    // 1. Recompute corridor signature
    let signature = corridor_service::recompute_signature(pool, game_id, is_safe)
        .await
        .map_err(|e| format!("Sig recompute failed: {e}"))?;

    crate::services::runtime_projection_service::rebuild_game_projection(pool, game_id)
        .await
        .map_err(|e| format!("Projection rebuild failed: {e}"))?;

    // 2. Refresh conflict cache
    let conflicts = metadata::get_active_mod_conflicts(pool, game_id).await?;

    // 3. KeyViewer Pipeline (Req-43)
    let emmm_data_dir = mods_path.join(".emmm_data");
    let keybinds_dir = emmm_data_dir.join("keybinds").join("active");
    let status_dir = emmm_data_dir.join("status");

    // Clean active artifacts (zero-leak policy)
    if keybinds_dir.exists() {
        let _ = std::fs::remove_dir_all(&keybinds_dir);
    }
    let _ = std::fs::create_dir_all(&keybinds_dir);
    let _ = std::fs::create_dir_all(&status_dir);

    // Harvest
    let enabled_mods = crate::repo::mod_repo::get_enabled_mods_paths(pool, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut active_hashes = std::collections::HashSet::new();
    let mut occurrence_counts = std::collections::HashMap::new();
    let mut hash_to_mod_path = std::collections::HashMap::new();
    let mut mod_keybinds = std::collections::HashMap::new();

    for mod_path_str in enabled_mods {
        let abs_path = mods_path.join(&mod_path_str);
        if let Ok(mod_hashes) = harvester::harvest_hashes_from_mod(&abs_path) {
            for (hash, occurrences) in mod_hashes {
                active_hashes.insert(hash.clone());
                *occurrence_counts.entry(hash.clone()).or_insert(0) += occurrences.len();
                hash_to_mod_path
                    .entry(hash)
                    .or_insert_with(Vec::new)
                    .push(mod_path_str.clone());
            }
        }
        if let Ok(keybinds) = harvester::harvest_keybinds_from_mod(&abs_path) {
            mod_keybinds.insert(mod_path_str, keybinds);
        }
    }

    // Load character entries from DB
    let db_objects = repo::object_repo::get_kv_matching_objects(pool, game_id)
        .await
        .map_err(|e| format!("Failed to load objects for KeyViewer: {e}"))?;

    let entries: Vec<resource_pack::KvObjectEntry> = db_objects
        .into_iter()
        .map(
            |(name, hash_db, _custom_skins)| resource_pack::KvObjectEntry {
                name,
                object_type: "Character".to_string(),
                code_hashes: hash_db
                    .0
                    .values()
                    .flat_map(|v| v.iter().map(|h| h.to_ascii_lowercase()))
                    .collect(),
                skin_hashes: hash_db
                    .0
                    .into_iter()
                    .map(|(s, h)| (s, h.into_iter().map(|h| h.to_ascii_lowercase()).collect()))
                    .collect(),
                tags: Vec::new(),
                thumbnail_path: None,
            },
        )
        .collect();

    // Match
    let config = matcher::MatchConfig::default();
    let matches = matcher::match_objects(&entries, &active_hashes, &occurrence_counts, &config);

    // Generate KeyViewer.ini
    let kv_ini_path = emmm_data_dir.join("KeyViewer.ini");
    generator::write_keyviewer_ini(
        &kv_ini_path,
        &matches,
        &ctx.settings.hotkeys.toggle_overlay,
        "keybinds/active",
    )?;

    // Map keybinds back to objects, grouped by mod source (Req-43)
    let mut sources_per_object = std::collections::HashMap::new();
    for m in &matches {
        let mut object_sources = Vec::new();
        let mut seen_mod_paths = std::collections::HashSet::new();

        for sentinel in &m.sentinel_hashes {
            if let Some(mod_paths) = hash_to_mod_path.get(sentinel) {
                for mp in mod_paths {
                    if seen_mod_paths.insert(mp) {
                        if let Some(kbs) = mod_keybinds.get(mp) {
                            // Use the folder name as the mod name
                            let mod_name = Path::new(mp)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| mp.clone());

                            object_sources.push(generator::SourceKeyBinding {
                                mod_name,
                                keybinds: kbs.clone(),
                            });
                        }
                    }
                }
            }
        }
        sources_per_object.insert(m.object_name.clone(), object_sources);
    }

    generator::write_keybind_files(
        &keybinds_dir,
        &matches,
        &sources_per_object,
        is_safe,
        ctx.settings.hotkeys.toggle_overlay.clone(),
    )?;

    // 4. Update Runtime Status (Req-42)
    let mut preset_name = None;
    if let Ok(snapshot) = corridor_service::get_corridor_state(pool, game_id, is_safe).await {
        if !snapshot.is_dirty && !snapshot.active_collection_is_unsaved {
            preset_name = snapshot.active_collection_name;
        }
    }

    let mut status = generator::StatusFields {
        safe_mode: is_safe,
        preset_name,
        folder_name: None,
        scope_name: None,
        conflict_count: Some(conflicts.len()),
    };

    // Override with fields from the mutation source if provided
    if let Some(overrides) = ctx.status_fields {
        if overrides.preset_name.is_some() {
            status.preset_name = overrides.preset_name;
        }
        if overrides.folder_name.is_some() {
            status.folder_name = overrides.folder_name;
        }
        if overrides.scope_name.is_some() {
            status.scope_name = overrides.scope_name;
        }
    }

    generator::write_status_file(&status_dir, &status, &ctx.settings.hotkeys)?;

    log::info!(
        "[post_apply] Completed post-apply tasks for game={}, sig={}",
        game_id,
        &signature[..8]
    );
    Ok(())
}

/// Convenience function to trigger a full overlay artifact regeneration for the active game.
/// Useful when settings (hotkeys, safe mode) change without a mod mutation.
pub async fn trigger_overlay_refresh_for_game(
    pool: &SqlitePool,
    config: &crate::services::config::ConfigService,
    suppressor: Arc<WatcherSuppressor>,
    game_id: &str,
) -> Result<(), String> {
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == game_id)
        .ok_or_else(|| format!("Game {} not found", game_id))?;
    let is_safe = settings.safe_mode.enabled;

    let ctx = PostApplyContext {
        pool: pool.clone(),
        game_id: game_id.to_string(),
        is_safe,
        mods_path: game.mod_path.clone(),
        suppressor,
        settings,
        status_fields: None,
    };

    run_post_apply_tasks(ctx).await
}

/// Convenience function to trigger a full overlay artifact regeneration for the active game.
/// Useful when settings (hotkeys, safe mode) change without a mod mutation.
pub async fn trigger_overlay_refresh(
    pool: &SqlitePool,
    config: &crate::services::config::ConfigService,
    suppressor: Arc<WatcherSuppressor>,
) -> Result<(), String> {
    let settings = config.get_settings();
    let game_id = settings.active_game_id.clone().ok_or("No active game")?;
    trigger_overlay_refresh_for_game(pool, config, suppressor, &game_id).await
}
