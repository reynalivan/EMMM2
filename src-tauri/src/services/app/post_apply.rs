use crate::domain::errors::AppError;
use crate::repo;
use crate::services::corridor_service;
use crate::services::hotkeys::HotkeyConfig;
use crate::services::keyviewer::generator;
use crate::services::keyviewer::harvester;
use crate::services::keyviewer::matcher;
use crate::services::mods::metadata;
use sqlx::SqlitePool;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

/// Context for post-mutation tasks.
#[derive(Clone)]
pub struct PostApplyContext {
    pub pool: SqlitePool,
    pub game_id: String,
    pub is_safe: bool,
    pub mods_path: PathBuf,
    /// Only the hotkey bindings, not the whole settings blob: post-apply reads
    /// `hotkeys` and nothing else, and this context is cloned on every
    /// mutation — a full `AppSettings` dragged every game, keyword and binding
    /// along with it.
    pub hotkeys: HotkeyConfig,
    /// Optional status overrides (e.g. preset name, folder name) from the mutation source.
    pub status_fields: Option<generator::StatusFields>,
}

fn cleanup_staging_after_error(staging: &std::path::Path, error: AppError) -> AppError {
    match std::fs::remove_dir_all(staging) {
        Ok(()) => error,
        Err(cleanup_error) => AppError::Io(format!(
            "KeyViewer artifact error ({error}); staging cleanup also failed ({cleanup_error}): {}",
            staging.display()
        )),
    }
}

/// Run tasks that should execute after any mod state change (Toggle, Apply, Switch).
///
/// Tasks include:
/// 1. Recomputing corridor signature (DB)
/// 2. Harvesting hashes from enabled mods
/// 3. Matching characters & generating KeyViewer.ini + keybind texts
/// 4. Refreshing conflict cache
/// 5. Updating runtime status banner
pub async fn run_post_apply_tasks(ctx: PostApplyContext) -> Result<(), AppError> {
    let pool = &ctx.pool;
    let game_id = &ctx.game_id;
    let is_safe = ctx.is_safe;
    let mods_path = &ctx.mods_path;

    log::info!(
        "[post_apply] Starting post-apply tasks for game={}",
        game_id
    );

    crate::repo::runtime_projection_repo::rebuild_game_projection(pool, game_id).await?;
    let game_type = crate::repo::game_repo::get_game_type(pool, game_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Game {game_id} not found")))?;

    // One query feeds both the conflict scan and the harvest below.
    let enabled_mods = crate::repo::mod_repo::get_enabled_mods_paths(pool, game_id).await?;

    // 2. Refresh conflict cache
    let conflicts = metadata::conflicts_for_enabled_paths(mods_path, &enabled_mods);

    // 3. KeyViewer Pipeline (Req-43)
    let emmm_data_dir = mods_path.join(".emmm_data");
    let keybinds_dir = emmm_data_dir.join("keybinds").join("active");
    let status_dir = emmm_data_dir.join("status");

    // Harvest: one pass per mod for both hashes and keybinds.
    let mut occurrence_counts = HashMap::new();
    let mut hash_to_mod_path = HashMap::new();
    let mut mod_keybinds = HashMap::new();

    for stored_path in enabled_mods {
        let abs_path = stored_path.resolve(mods_path);
        let harvest = harvester::harvest_mod(&abs_path)?;

        for (hash, occurrences) in harvest.hashes {
            *occurrence_counts.entry(hash.clone()).or_insert(0) += occurrences.len();
            hash_to_mod_path
                .entry(hash)
                .or_insert_with(Vec::new)
                .push(stored_path.clone());
        }
        mod_keybinds.insert(stored_path, harvest.keybinds);
    }

    // Load character entries from DB
    let db_objects = repo::object_repo::get_kv_matching_objects(pool, game_id).await?;

    let entries: Vec<matcher::KvObjectEntry> = db_objects
        .into_iter()
        .map(|(name, hash_db)| {
            // Lowercase once: `code_hashes` is the flattened `skin_hashes`, and
            // building them independently case-folded every hash twice.
            let skin_hashes: HashMap<String, Vec<String>> = hash_db
                .0
                .into_iter()
                .map(|(skin, hashes)| {
                    let mut folded: Vec<String> = hashes
                        .into_iter()
                        .map(|hash| hash.trim_start_matches("0x").to_ascii_lowercase())
                        .filter(|hash| {
                            hash.len() == 8
                                && hash.chars().all(|character| character.is_ascii_hexdigit())
                        })
                        .collect();
                    folded.sort_unstable();
                    folded.dedup();
                    (skin, folded)
                })
                .collect();
            let code_hashes: Vec<String> = skin_hashes
                .values()
                .flatten()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();

            matcher::KvObjectEntry {
                name,
                object_type: "Character".to_string(),
                code_hashes,
                skin_hashes,
                tags: Vec::new(),
                thumbnail_path: None,
            }
        })
        .collect();

    // Match
    let config = matcher::MatchConfig::default();
    let active_hashes: HashSet<String> = occurrence_counts.keys().cloned().collect();
    let matches = matcher::match_objects(&entries, &active_hashes, &occurrence_counts, &config);
    let kv_ini_content =
        generator::generate_keyviewer_ini(&matches, &ctx.hotkeys.toggle_overlay, game_type)?;

    // Map keybinds back to objects, grouped by mod source (Req-43)
    let mut sources_per_object = HashMap::new();
    for m in &matches {
        let mut object_sources = Vec::new();
        let mut seen_mod_paths = HashSet::new();

        for sentinel in &m.sentinel_hashes {
            if let Some(mod_paths) = hash_to_mod_path.get(sentinel) {
                for mp in mod_paths {
                    if seen_mod_paths.insert(mp) {
                        if let Some(kbs) = mod_keybinds.get(mp) {
                            // Use the folder name as the mod name
                            let mod_name = mp.folder_name().to_string();

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

    let staging_keybinds = generator::create_staging_directory(&keybinds_dir)?;
    if let Err(write_error) = generator::write_keybind_files(
        &staging_keybinds,
        &matches,
        &sources_per_object,
        &ctx.hotkeys.toggle_overlay,
    ) {
        return Err(cleanup_staging_after_error(&staging_keybinds, write_error));
    }

    // 4. Update Runtime Status (Req-42)
    //
    // `get_corridor_state` re-derives the whole live runtime state — a
    // per-mod filesystem classify pass. A caller that already settled the
    // corridor (the apply pipeline) supplies the answer instead.
    let caller_knows_preset = ctx
        .status_fields
        .as_ref()
        .is_some_and(|fields| fields.preset_name.is_some());

    let mut preset_name = None;
    if !caller_knows_preset {
        match corridor_service::get_corridor_state(
            pool,
            game_id,
            crate::domain::corridor::Corridor::from_is_safe(is_safe),
        )
        .await
        {
            Ok(snapshot) if !snapshot.is_dirty => {
                preset_name = snapshot.active_collection_name;
            }
            Ok(_) => {}
            Err(error) => log::warn!("[post_apply] Could not derive corridor status: {error}"),
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

    if let Err(error) = generator::write_status_file(&status_dir, &status, &ctx.hotkeys) {
        return Err(cleanup_staging_after_error(&staging_keybinds, error));
    }
    generator::replace_directory(&staging_keybinds, &keybinds_dir)?;
    generator::atomic_write(&emmm_data_dir.join("KeyViewer.ini"), &kv_ini_content)?;

    log::info!(
        "[post_apply] Completed post-apply tasks for game={}",
        game_id
    );
    Ok(())
}

/// Convenience function to trigger a full overlay artifact regeneration for the active game.
/// Useful when settings (hotkeys, safe mode) change without a mod mutation.
pub async fn trigger_overlay_refresh_for_game(
    pool: &SqlitePool,
    config: &crate::services::config::ConfigService,
    game_id: &str,
) -> Result<(), AppError> {
    let settings = config.get_settings();
    let game = settings
        .games
        .iter()
        .find(|entry| entry.id == game_id)
        .ok_or_else(|| AppError::Internal(format!("Game {} not found", game_id)))?;
    let is_safe = settings.safe_mode.enabled;

    let ctx = PostApplyContext {
        pool: pool.clone(),
        game_id: game_id.to_string(),
        is_safe,
        mods_path: game.mod_path.clone(),
        hotkeys: settings.hotkeys.clone(),
        status_fields: None,
    };

    run_post_apply_tasks(ctx).await
}

/// Convenience function to trigger a full overlay artifact regeneration for the active game.
/// Useful when settings (hotkeys, safe mode) change without a mod mutation.
pub async fn trigger_overlay_refresh(
    pool: &SqlitePool,
    config: &crate::services::config::ConfigService,
) -> Result<(), AppError> {
    let game_id = config
        .with_settings(|settings| settings.active_game_id.clone())
        .ok_or_else(|| AppError::Internal("No active game".to_string()))?;
    trigger_overlay_refresh_for_game(pool, config, &game_id).await
}
