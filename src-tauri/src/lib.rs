#[cfg(not(test))]
use tauri::Manager;
#[cfg(not(test))]
use tauri_plugin_log::{Target, TargetKind};

pub mod commands;
pub mod database;
pub mod domain;
pub mod pipeline;
pub mod repo;
pub mod services;
#[cfg(test)]
pub mod test_utils;
pub mod types;

/// Standard prefix for disabled mod folders. Shared across commands.
pub const DISABLED_PREFIX: &str = "DISABLED ";

macro_rules! emmm_collect_commands {
    () => {
        tauri_specta::collect_commands![
            commands::app::app_cmds::check_config_status,
            commands::app::app_cmds::close_splashscreen,
            commands::app::dashboard_cmds::get_dashboard_stats,
            commands::app::dashboard_cmds::get_active_keybindings,
            commands::app::workspace_cmds::get_workspace_view_model,
            commands::app::workspace_cmds::execute_workspace_switch,
            commands::app::app_cmds::get_logs,
            commands::app::app_cmds::open_log_folder,
            commands::app::app_cmds::reset_database,
            commands::app::app_cmds::check_path_exists_cmd,
            commands::app::app_cmds::ensure_dir_cmd,
            commands::app::game_cmds::auto_detect_games,
            commands::app::game_cmds::add_game_manual,
            commands::app::game_cmds::save_onboarding_games,
            commands::app::game_cmds::get_games,
            commands::app::game_cmds::launch_game,
            commands::objects::master_db_cmds::get_game_schema,
            commands::objects::master_db_cmds::get_object,
            commands::objects::master_db_cmds::get_master_db,
            commands::objects::master_db_cmds::search_master_db,
            commands::objects::master_db_cmds::match_object_with_db,
            commands::objects::master_db_cmds::pin_object,
            commands::scanner::scan_control_cmds::cancel_scan_cmd,
            commands::scanner::archive_cmds::detect_archives_cmd,
            commands::scanner::archive_cmds::extract_archive_cmd,
            commands::scanner::archive_cmds::analyze_archive_cmd,
            commands::scanner::archive_cmds::match_check_folder_cmd,
            commands::scanner::archive_cmds::abort_extraction_cmd,
            commands::scanner::conflict_cmds::detect_conflicts_cmd,
            commands::scanner::conflict_cmds::detect_conflicts_in_folder_cmd,
            commands::scanner::watcher_cmds::set_watcher_suppression,
            commands::folder_grid::get_mod_thumbnail,
            commands::mods::mod_core_cmds::open_in_explorer,
            commands::mods::mod_core_cmds::reveal_object_in_explorer,
            commands::mods::conflict_cmds::resolve_conflict,
            commands::mods::conflict_cmds::get_conflict_details,
            commands::mods::conflict_cmds::ignore_object_conflict,
            commands::mods::conflict_cmds::revoke_object_conflict,
            commands::mods::conflict_cmds::list_ignored_object_conflicts,
            commands::mods::mod_core_cmds::rename_mod_folder,
            commands::mods::mod_import_cmds::import_mods_from_paths,
            commands::mods::mod_import_cmds::ingest_dropped_folders,
            commands::mods::mod_bulk_cmds::bulk_toggle_mods,
            commands::mods::mod_bulk_cmds::bulk_delete_mods,
            commands::mods::mod_bulk_cmds::bulk_update_info,
            commands::mods::mod_bulk_cmds::bulk_toggle_favorite,
            commands::mods::mod_bulk_cmds::bulk_pin_mods,
            commands::mods::mod_meta_cmds::toggle_mod_safe,
            commands::mods::mod_meta_cmds::suggest_random_mods,
            commands::mods::mod_meta_cmds::get_active_mod_conflicts,
            commands::mods::mod_meta_cmds::read_mod_info,
            commands::mods::mod_meta_cmds::update_mod_info,
            commands::mods::mod_meta_cmds::set_mod_category,
            commands::mods::mod_meta_cmds::set_object_mods_category,
            commands::mods::mod_meta_cmds::list_move_targets_for_object,
            commands::mods::mod_meta_cmds::move_mod_to_object,
            commands::mods::mod_meta_cmds::move_mods_to_object,
            commands::mods::mod_thumbnail_cmds::update_mod_thumbnail,
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
            commands::app::settings_cmds::set_active_game,
            commands::app::settings_cmds::set_auto_close_launcher,
            commands::app::settings_cmds::run_maintenance,
            commands::app::settings_cmds::reset_pin_with_recovery_code,
            commands::app::settings_cmds::clear_old_thumbnails,
            commands::app::theme_cmds::list_custom_themes,
            commands::app::theme_cmds::load_custom_theme,
            commands::app::theme_cmds::save_custom_theme,
            commands::app::theme_cmds::delete_custom_theme,
            commands::objects::object_cmds::get_objects_cmd,
            commands::objects::object_cmds::get_category_counts_cmd,
            commands::objects::object_cmds::create_object_cmd,
            commands::objects::object_cmds::update_object_cmd,
            commands::objects::object_cmds::apply_object_match_cmd,
            commands::objects::object_cmds::delete_object_cmd,
            commands::collections::cmds::switch_corridor,
            commands::collections::cmds::preview_corridor_switch,
            commands::collections::cmds::get_corridor_state,
            commands::collections::cmds::get_apply_progress,
            commands::collections::cmds::list_collections,
            commands::collections::cmds::create_collection,
            commands::collections::cmds::apply_collection,
            commands::collections::cmds::update_collection,
            commands::collections::cmds::replace_collection_with_current_state,
            commands::collections::cmds::delete_collection,
            commands::collections::cmds::app_startup_check,
            commands::collections::cmds::check_boot_security,
            commands::collections::cmds::resolve_recovery_task,
            commands::collections::cmds::get_collection_preview,
            commands::collections::cmds::preview_apply_collection,
            commands::collections::cmds::has_pin,
            commands::collections::cmds::set_pin,
            commands::collections::cmds::verify_pin,
            commands::collections::cmds::clear_pin,
            commands::collections::cmds::get_pin_status,
            commands::scanner::deepmatch_scanner_cmds::deepmatch_scanner_cmd,
            commands::scanner::deepmatch_scanner_cmds::deepmatch_preview_cmd,
            commands::scanner::deepmatch_scanner_cmds::deepmatch_preview_for_objects_cmd,
            commands::scanner::deepmatch_scanner_cmds::commit_scan_cmd,
            commands::scanner::deepmatch_scanner_cmds::score_candidates_batch_cmd,
            commands::scanner::deepmatch_scanner_cmds::list_folder_entries_cmd,
            commands::scanner::disk_reconcile_cmds::reconcile_disk_state_cmd,
            commands::scanner::watcher_cmds::start_watcher,
            commands::scanner::watcher_cmds::stop_watcher,
            commands::duplicates::dup_scan_cmds::dup_scan_start,
            commands::duplicates::dup_scan_cmds::dup_scan_cancel,
            commands::duplicates::dup_scan_cmds::dup_scan_get_report,
            commands::duplicates::dup_resolve_cmds::dup_resolve_batch,
            commands::duplicates::dup_ignore_cmds::get_ignored_pairs,
            commands::duplicates::dup_ignore_cmds::remove_ignored_pair,
            commands::app::update_cmds::check_metadata_update,
            commands::app::update_cmds::fetch_missing_asset,
            commands::app::hotkey_cmds::update_hotkey_config,
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
            commands::browser::browser_cmds::browser_list_import_queue,
            commands::browser::browser_cmds::browser_confirm_import,
            commands::browser::browser_cmds::browser_cancel_import,
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[cfg(not(test))]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new().commands(emmm_collect_commands!());

    /*
    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/bindings.ts",
        )
        .expect("The types could not be exported");
    */

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())

        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(hotkey_manager) = app.try_state::<services::hotkeys::manager::HotkeyManager>() {
                        hotkey_manager.inner().on_shortcut_pressed(app, &shortcut.to_string());
                    }
                })
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("emmm.log".into()),
                    }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .manage(services::scanner::watcher::WatcherState::new())
        .manage(services::disk_reconcile::orchestrator::DiskReconcileState::new())
        .setup(move |app| {
            let app_handle = app.handle();

            if let Some(window) = app_handle.get_webview_window("main") {
                if let Ok(outer_pos) = window.outer_position() {
                    let mut is_visible = false;
                    if let Ok(monitors) = window.available_monitors() {
                        for monitor in monitors {
                            let m_pos = monitor.position();
                            let m_size = monitor.size();
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

            #[cfg(desktop)]
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;

            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                services::images::thumbnail_cache::ThumbnailCache::init(&app_data_dir);

                #[cfg(desktop)]
                {
                    use tauri::async_runtime::block_on;
                    let db_path = app_data_dir.join("app.db");
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
                                log::error!("Database connection or migration failed: {e}. Attempting recovery...");
                                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                let backup_path = app_data_dir.join(format!("app_corrupt_{}.db", timestamp));
                                let _ = std::fs::rename(&db_path, &backup_path);
                                let recovered_pool = try_init().await.expect("Failed to initialize database after recovery");
                                recovered_pool
                            }
                        };

                        if let Err(e) = services::scanner::sync::migrate_to_stable_ids(&p).await {
                            log::warn!("Stable ID migration skipped: {e}");
                        }
                        if let Err(e) = repo::unicode_keys::ensure_unicode_keys(&p).await {
                            log::warn!("Unicode key backfill skipped: {e}");
                        }
                        p
                    });
                    app.manage(pool);
                }
            }

            let pool_ref: tauri::State<'_, sqlx::SqlitePool> = app.state();
            app.manage(services::config::ConfigService::init(
                app_handle,
                pool_ref.inner().clone(),
            ));

            let config_ref: tauri::State<'_, services::config::ConfigService> = app.state();
            let hotkey_config = config_ref.get_settings().hotkeys;
            match services::hotkeys::manager::HotkeyManager::new(&hotkey_config) {
                Ok(hk_manager) => {
                    let _ = hk_manager.update_bindings(app_handle, &hotkey_config);
                    app.manage(hk_manager);
                }
                Err(_) => {
                    let disabled_config = services::hotkeys::HotkeyConfig { enabled: false, ..Default::default() };
                    if let Ok(mgr) = services::hotkeys::manager::HotkeyManager::new(&disabled_config) {
                        app.manage(mgr);
                    }
                }
            }

            {
                let config_svc: tauri::State<'_, services::config::ConfigService> = app.state();
                let settings = config_svc.get_settings();
                let pool_state: tauri::State<'_, sqlx::SqlitePool> = app.state();
                let pool_clone = pool_state.inner().clone();
                let watcher_state: tauri::State<'_, services::scanner::watcher::WatcherState> =
                    app.state();
                let disk_reconcile_state: tauri::State<
                    '_,
                    services::disk_reconcile::orchestrator::DiskReconcileState,
                > = app.state();
                use tauri::async_runtime::block_on;
                block_on(async {
                    match sqlx::query("DELETE FROM tasks WHERE created_at < datetime('now', '-7 days')")
                        .execute(&pool_clone)
                        .await
                    {
                        Ok(result) if result.rows_affected() > 0 => {
                            log::info!(
                                "startup: purged {} old task log(s) before boot reconcile",
                                result.rows_affected()
                            );
                        }
                        Ok(_) => {}
                        Err(error) => {
                            log::warn!("startup: task GC failed before boot reconcile: {error}");
                        }
                    }

                    let Some(active_game_id) = settings.active_game_id.as_deref() else {
                        return;
                    };
                    let Some(game) = settings.games.iter().find(|entry| entry.id == active_game_id) else {
                        return;
                    };
                    let mod_path = game.mod_path.to_string_lossy().to_string();
                    if mod_path.is_empty() {
                        return;
                    }

                    match services::disk_reconcile::orchestrator::reconcile_disk_state(
                        services::disk_reconcile::orchestrator::DiskReconcileContext {
                            pool: &pool_clone,
                            config: config_svc.inner(),
                            state: &disk_reconcile_state,
                            watcher_suppressor: watcher_state.suppressor.clone(),
                        },
                        services::disk_reconcile::orchestrator::DiskReconcileRequest::manual(
                            game.id.clone(),
                            services::disk_reconcile::types::DiskReconcileReason::StartupBoot,
                            Vec::new(),
                            true,
                        ),
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(error) => {
                            log::warn!(
                                "Startup Disk Reconcile failed for '{}': {}",
                                game.name,
                                error
                            );
                        }
                    }
                });
            }

            Ok(())
        })
        .manage(commands::scanner::scan_control_cmds::ScanState::new())
        .manage(commands::duplicates::dup_scan_cmds::DupScanState::new())
        .manage(services::fs_utils::operation_lock::OperationLock::new())
        .manage(commands::objects::master_db_cmds::MasterDbCache::new())
        .manage(commands::scanner::archive_cmds::ExtractionState::new())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
pub fn run() {}

#[cfg(test)]
mod specta_tests {
    use super::*;

    #[test]
    fn export_bindings() {
        let output_path = std::path::Path::new("target").join("specta-bindings.generated.ts");
        tauri_specta::Builder::<tauri::Wry>::new()
            .commands(emmm_collect_commands!())
            .export(
                specta_typescript::Typescript::default()
                    .bigint(specta_typescript::BigIntExportBehavior::Number),
                output_path,
            )
            .expect("The types could not be exported");
    }
}
