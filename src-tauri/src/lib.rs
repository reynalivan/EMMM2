use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

pub mod commands;
pub mod database;
pub mod services;
#[cfg(test)]
pub mod test_utils;
pub mod types;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
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

            // TC-01: Multi-Monitor Bounds Check
            // If the window was saved on a monitor that is no longer connected, it might spawn off-screen.
            if let Some(window) = app_handle.get_webview_window("main") {
                if let Ok(outer_pos) = window.outer_position() {
                    let mut is_visible = false;
                    if let Ok(monitors) = window.available_monitors() {
                        for monitor in monitors {
                            let m_pos = monitor.position();
                            let m_size = monitor.size();
                            // Check if top-left corner is within this monitor's bounds
                            if outer_pos.x >= m_pos.x
                                && outer_pos.x < m_pos.x + m_size.width as i32
                                && outer_pos.y >= m_pos.y
                                && outer_pos.y < m_pos.y + m_size.height as i32
                            {
                                is_visible = true;
                                break;
                            }
                        }
                    }
                    if !is_visible {
                        log::warn!("Window spawned off-screen (disconnected monitor). Centering on primary.");
                        let _ = window.center();
                    }
                }
            }

            // Epic 12: Register app updater plugin
            #[cfg(desktop)]
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;

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

                        let try_init = || async {
                            let opts = SqliteConnectOptions::new()
                                .filename(&db_path)
                                .create_if_missing(true);

                            let pool = SqlitePoolOptions::new()
                                .max_connections(5)
                                .connect_with(opts)
                                .await?;

                            sqlx::migrate!("./migrations")
                                .run(&pool)
                                .await?;

                            Ok::<sqlx::SqlitePool, sqlx::Error>(pool)
                        };

                        let p = match try_init().await {
                            Ok(pool) => pool,
                            Err(e) => {
                                log::error!("Database connection or migration failed: {e}. Attempting recovery by recreating DB...");
                                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                let backup_path = app_data_dir.join(format!("app_corrupt_{}.db", timestamp));
                                if let Err(be) = std::fs::rename(&db_path, &backup_path) {
                                    log::error!("Failed to backup corrupt db: {be}");
                                } else {
                                    log::info!("Backed up corrupt db to {}", backup_path.display());
                                }

                                try_init().await.expect("Failed to initialize database after recovery attempt")
                            }
                        };

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
            app.manage(services::config::ConfigService::init(
                app_handle,
                pool_ref.inner().clone(),
            ));

            // Initialize HotkeyManager with config from ConfigService
            let config_ref: tauri::State<'_, services::config::ConfigService> = app.state();
            let hotkey_config = config_ref.get_settings().hotkeys;
            match services::hotkeys::manager::HotkeyManager::new(&hotkey_config) {
                Ok(hk_manager) => {
                    app.manage(hk_manager);
                    log::info!("HotkeyManager initialized");
                }
                Err(e) => {
                    log::warn!("Failed to initialize HotkeyManager: {e}. Hotkeys disabled.");
                    // Create a disabled manager so the state is still available
                    let disabled_config = services::hotkeys::HotkeyConfig {
                        enabled: false,
                        ..Default::default()
                    };
                    if let Ok(mgr) = services::hotkeys::manager::HotkeyManager::new(&disabled_config) {
                        app.manage(mgr);
                    }
                }
            }

            Ok(())
        })
        .manage(commands::scanner::scan_cmds::ScanState::new())
        .manage(commands::duplicates::dup_scan_cmds::DupScanState::new())
        .manage(services::scanner::watcher::WatcherState::new())
        .manage(services::fs_utils::operation_lock::OperationLock::new())
        .manage(commands::objects::master_db_cmds::MasterDbCache::new())
        .invoke_handler(tauri::generate_handler![
            commands::app::app_cmds::check_config_status,
            commands::app::dashboard_cmds::get_dashboard_stats,
            commands::app::dashboard_cmds::get_active_keybindings,
            commands::app::app_cmds::get_log_lines,
            commands::app::app_cmds::open_log_folder,
            commands::app::app_cmds::reset_database,
            commands::app::game_cmds::auto_detect_games,
            commands::app::game_cmds::add_game_manual,
            commands::app::game_cmds::remove_game,
            commands::app::game_cmds::get_games,
            commands::app::game_cmds::launch_game,
            commands::objects::master_db_cmds::get_game_schema,
            commands::objects::master_db_cmds::get_object,
            commands::objects::master_db_cmds::get_master_db,
            commands::objects::master_db_cmds::search_master_db,
            commands::objects::master_db_cmds::match_object_with_db,
            commands::objects::master_db_cmds::pin_object,
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
            commands::folder_grid::list_mod_folders,
            commands::folder_grid::get_mod_thumbnail,
            commands::mods::mod_core_cmds::open_in_explorer,
            commands::mods::mod_core_cmds::reveal_object_in_explorer,
            commands::mods::mod_core_cmds::toggle_mod,
            commands::mods::conflict_cmds::resolve_conflict,
            commands::mods::conflict_cmds::get_conflict_details,
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
            commands::mods::mod_meta_cmds::toggle_mod_safe,
            commands::mods::mod_meta_cmds::suggest_random_mods,
            commands::mods::mod_meta_cmds::get_active_mod_conflicts,
            commands::mods::mod_meta_cmds::read_mod_info,
            commands::mods::mod_meta_cmds::update_mod_info,
            commands::mods::mod_meta_cmds::set_mod_category,
            commands::mods::mod_meta_cmds::move_mod_to_object,
            commands::mods::mod_thumbnail_cmds::update_mod_thumbnail,
            commands::mods::mod_thumbnail_cmds::get_thumbnail,
            commands::mods::mod_thumbnail_cmds::paste_thumbnail,
            commands::folder_grid::delete_mod_thumbnail,
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
            commands::app::settings_cmds::set_auto_close_launcher,
            commands::app::settings_cmds::run_maintenance,
            commands::app::settings_cmds::clear_old_thumbnails,
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
            commands::collections::collection_cmds::undo_collection,
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
            // commands::duplicates::dup_resolve_cmds::dup_resolve_batch,
            commands::app::update_cmds::check_metadata_update,
            commands::app::update_cmds::fetch_missing_asset,
            commands::app::hotkey_cmds::get_hotkey_bindings,
            commands::app::hotkey_cmds::detect_hotkey_conflicts,
            commands::app::hotkey_cmds::update_hotkey_config,
            // Epic 44: Discover Hub + In-App Browser + Auto Smart Import
            commands::browser::browser_cmds::browser_open_tab,
            commands::browser::browser_cmds::browser_navigate,
            commands::browser::browser_cmds::browser_reload_tab,
            commands::browser::browser_cmds::browser_clear_data,
            commands::browser::browser_cmds::browser_get_homepage,
            commands::browser::browser_cmds::browser_set_homepage,
            commands::browser::browser_cmds::browser_list_downloads,
            commands::browser::browser_cmds::browser_cancel_download,
            commands::browser::browser_cmds::browser_delete_download,
            commands::browser::browser_cmds::browser_clear_imported,
            commands::browser::browser_cmds::browser_clear_old_downloads,
            commands::browser::browser_cmds::browser_import_selected,
            commands::browser::browser_cmds::import_get_queue,
            commands::browser::browser_cmds::import_confirm_review,
            commands::browser::browser_cmds::import_skip,
            commands::browser::browser_cmds::create_download_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
