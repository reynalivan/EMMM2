use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

pub mod commands;
pub mod database;
pub mod services;
pub mod types;
#[cfg(test)]
pub mod test_utils;

/// Standard prefix for disabled mod folders. Shared across commands.
pub const DISABLED_PREFIX: &str = "DISABLED ";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is attempted
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        // tauri_plugin_store removed: all settings now persisted in SQLite
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("emmm2.log".into()),
                    }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .setup(move |app| {
            let app_handle = app.handle();
            // Initialize services
            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                services::images::thumbnail_cache::ThumbnailCache::init(&app_data_dir);

                // Initialize Backend SQLx Pool (shared with plugin)
                #[cfg(desktop)]
                {
                    use tauri::async_runtime::block_on;
                    let db_path = app_data_dir.join("app.db");
                    // Ensure directory exists
                    if !app_data_dir.exists() {
                        let _ = std::fs::create_dir_all(&app_data_dir);
                    }

                    let pool = block_on(async {
                        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
                        let opts = SqliteConnectOptions::new()
                            .filename(&db_path)
                            .create_if_missing(true);
                        let p = SqlitePoolOptions::new()
                            .max_connections(5)
                            .connect_with(opts)
                            .await
                            .expect("failed to connect to backend db");

                        // Run standard sqlx migrations (compiled into the binary)
                        sqlx::migrate!("./migrations")
                            .run(&p)
                            .await
                            .expect("Failed to run database migrations");

                        // One-time migration: UUID → stable BLAKE3 IDs for mods
                        if let Err(e) = services::scanner::sync::migrate_to_stable_ids(&p).await {
                            log::warn!("Stable ID migration skipped: {e}");
                        }

                        p
                    });
                    app.manage(pool);
                }
            }
            // Initialize ConfigService
            let pool_ref: tauri::State<'_, sqlx::SqlitePool> = app.state();
            app.manage(services::config::ConfigService::init(app_handle, pool_ref.inner().clone()));

            Ok(())
        })
        .manage(commands::scanner::scan_cmds::ScanState::new())
        .manage(commands::duplicates::dup_scan_cmds::DupScanState::new())
        .manage(services::scanner::watcher::WatcherState::new())
        .manage(services::core::operation_lock::OperationLock::new())
        .invoke_handler(tauri::generate_handler![
            commands::app::app_cmds::check_config_status,
            commands::app::app_cmds::get_log_lines,
            commands::app::app_cmds::open_log_folder,
            commands::app::app_cmds::reset_database,
            commands::app::game_cmds::auto_detect_games,
            commands::app::game_cmds::add_game_manual,
            commands::app::game_cmds::remove_game,
            commands::app::game_cmds::get_games,
            commands::app::game_cmds::launch_game,
            commands::mods::object_cmds::get_game_schema,
            commands::mods::object_cmds::get_object,
            commands::mods::object_cmds::get_master_db,
            commands::mods::object_cmds::match_object_with_db,
            commands::mods::object_cmds::pin_object,
            commands::scanner::scan_cmds::cancel_scan_cmd,
            commands::scanner::archive_cmds::detect_archives_cmd,
            commands::scanner::archive_cmds::extract_archive_cmd,
            commands::scanner::archive_cmds::analyze_archive_cmd,
            commands::scanner::scan_cmds::start_scan,
            commands::scanner::organize_cmds::auto_organize_mods,
            commands::scanner::scan_cmds::get_scan_result,
            commands::scanner::conflict_cmds::detect_conflicts_cmd,
            commands::scanner::conflict_cmds::detect_conflicts_in_folder_cmd,
            commands::scanner::watcher_cmds::set_watcher_suppression_cmd,
            commands::explorer::list_mod_folders,
            commands::explorer::get_mod_thumbnail,
            commands::mods::mod_core_cmds::open_in_explorer,
            commands::mods::mod_core_cmds::reveal_object_in_explorer,
            commands::mods::mod_core_cmds::toggle_mod,
            commands::mods::mod_core_cmds::rename_mod_folder,
            commands::mods::mod_core_cmds::pre_delete_check,
            commands::mods::mod_import_cmds::import_mods_from_paths,
            commands::mods::mod_import_cmds::ingest_dropped_folders,
            commands::mods::mod_bulk_cmds::bulk_toggle_mods,
            commands::mods::mod_bulk_cmds::bulk_delete_mods,
            commands::mods::mod_bulk_cmds::bulk_update_info,
            commands::mods::mod_bulk_cmds::bulk_toggle_favorite,
            commands::mods::mod_bulk_cmds::bulk_pin_mods,
            commands::mods::mod_meta_cmds::repair_orphan_mods,
            commands::mods::mod_meta_cmds::pin_mod,
            commands::mods::mod_meta_cmds::toggle_favorite,
            commands::mods::mod_meta_cmds::pick_random_mod,
            commands::mods::mod_meta_cmds::get_active_mod_conflicts,
            commands::mods::mod_meta_cmds::read_mod_info,
            commands::mods::mod_meta_cmds::update_mod_info,
            commands::mods::mod_meta_cmds::set_mod_category,
            commands::mods::mod_meta_cmds::move_mod_to_object,
            commands::mods::mod_thumbnail_cmds::update_mod_thumbnail,
            commands::mods::mod_thumbnail_cmds::get_thumbnail,
            commands::mods::mod_thumbnail_cmds::paste_thumbnail,
            commands::explorer::delete_mod_thumbnail,
            commands::mods::trash_cmds::delete_mod,
            commands::mods::trash_cmds::restore_mod,
            commands::mods::trash_cmds::list_trash,
            commands::mods::trash_cmds::empty_trash,
            commands::mods::preview_cmds::list_mod_ini_files,
            commands::mods::preview_cmds::read_mod_ini,
            commands::mods::preview_cmds::write_mod_ini,
            commands::mods::preview_cmds::list_mod_preview_images,
            commands::mods::preview_cmds::save_mod_preview_image,
            commands::mods::preview_cmds::remove_mod_preview_image,
            commands::mods::preview_cmds::clear_mod_preview_images,
            commands::app::settings_cmds::get_settings,
            commands::app::settings_cmds::save_settings,
            commands::app::settings_cmds::set_safe_mode_pin,
            commands::app::settings_cmds::verify_pin,
            commands::app::settings_cmds::set_active_game,
            commands::app::settings_cmds::set_safe_mode_enabled,
            commands::app::settings_cmds::run_maintenance,
            commands::scanner::conflict_cmds::enable_only_this,
            commands::scanner::conflict_cmds::check_duplicate_enabled,
            commands::scanner::conflict_cmds::check_shader_conflicts,
            commands::collections::collection_cmds::list_collections,
            commands::collections::collection_cmds::create_collection,
            commands::objects::object_cmds::get_objects_cmd,
            commands::objects::object_cmds::get_category_counts_cmd,
            commands::objects::object_cmds::create_object_cmd,
            commands::objects::object_cmds::update_object_cmd,
            commands::objects::object_cmds::delete_object_cmd,
            commands::collections::collection_cmds::update_collection,
            commands::collections::collection_cmds::delete_collection,
            commands::collections::collection_cmds::apply_collection,
            commands::collections::collection_cmds::get_collection_preview,
            commands::scanner::sync_cmds::sync_database_cmd,
            commands::scanner::sync_cmds::scan_preview_cmd,
            commands::scanner::sync_cmds::commit_scan_cmd,
            commands::scanner::sync_cmds::score_candidates_batch_cmd,
            commands::scanner::sync_cmds::list_folder_entries_cmd,
            commands::scanner::watcher_cmds::start_watcher_cmd,
            commands::duplicates::dup_scan_cmds::dup_scan_start,
            commands::duplicates::dup_scan_cmds::dup_scan_cancel,
            commands::duplicates::dup_scan_cmds::dup_scan_get_report,
            commands::duplicates::dup_resolve_cmds::dup_resolve_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
