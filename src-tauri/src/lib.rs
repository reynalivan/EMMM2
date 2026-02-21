use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_sql::{Migration, MigrationKind};

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
    let migrations = vec![
        Migration {
            version: 1,
            description: "create_initial_tables",
            sql: include_str!("../migrations/001_init.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "epic1_fixes",
            sql: include_str!("../migrations/002_epic1_fixes.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "epic2_scanner",
            sql: include_str!("../migrations/003_epic2_scanner.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "epic3_objects",
            sql: include_str!("../migrations/004_epic3_objects.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "epic4_explorer",
            sql: include_str!("../migrations/005_epic4_explorer.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 6,
            description: "objects_pin",
            sql: include_str!("../migrations/006_objects_pin.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 7,
            description: "drop_trash_log",
            sql: include_str!("../migrations/007_drop_trash_log.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 8,
            description: "add_favorite_column",
            sql: include_str!("../migrations/008_add_favorite_column.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 9,
            description: "epic9_dedup_scanner",
            sql: include_str!("../migrations/20260216103000_epic9_dedup_scanner.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 10,
            description: "epic8_collections_indexes",
            sql: include_str!("../migrations/20260216143000_epic8_collections.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 11,
            description: "add_sync_mode",
            sql: include_str!("../migrations/20260217120000_add_sync_mode.sql"),
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is attempted
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(tauri_plugin_store::Builder::new().build())
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
        .setup(|app| {
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
                        SqlitePoolOptions::new()
                            .max_connections(5)
                            .connect_with(opts)
                            .await
                            .expect("failed to connect to backend db")
                    });
                    app.manage(pool);
                }
            }
            // Initialize ConfigService
            app.manage(services::config::ConfigService::init(app_handle));

            Ok(())
        })
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:app.db", migrations)
                .build(),
        )
        .manage(commands::scan_cmds::ScanState::new())
        .manage(commands::dup_scan_cmds::DupScanState::new())
        .manage(services::watcher::WatcherState::new())
        .manage(services::operation_lock::OperationLock::new())
        .manage(services::collections::CollectionsUndoState::new())
        .invoke_handler(tauri::generate_handler![
            commands::app_cmds::check_config_status,
            commands::app_cmds::get_log_lines,
            commands::app_cmds::open_log_folder,
            commands::game_cmds::auto_detect_games,
            commands::game_cmds::add_game_manual,
            commands::game_cmds::get_games,
            commands::game_cmds::launch_game,
            commands::object_cmds::get_game_schema,
            commands::object_cmds::get_object,
            commands::object_cmds::get_master_db,
            commands::object_cmds::match_object_with_db,
            commands::object_cmds::pin_object,
            commands::object_cmds::delete_object,
            commands::scan_cmds::cancel_scan_cmd,
            commands::scan_cmds::detect_archives_cmd,
            commands::scan_cmds::extract_archive_cmd,
            commands::scan_cmds::analyze_archive_cmd,
            commands::scan_cmds::start_scan,
            commands::scan_cmds::auto_organize_mods,
            commands::scan_cmds::get_scan_result,
            commands::scan_cmds::detect_conflicts_cmd,
            commands::scan_cmds::detect_conflicts_in_folder_cmd,
            commands::folder_cmds::list_mod_folders,
            commands::folder_cmds::get_mod_thumbnail,
            commands::mod_cmds::open_in_explorer,
            commands::mod_cmds::toggle_mod,
            commands::mod_cmds::delete_mod,
            commands::mod_cmds::pin_mod,
            commands::mod_cmds::toggle_favorite,
            commands::mod_cmds::pick_random_mod,
            commands::mod_cmds::get_active_mod_conflicts,
            commands::mod_cmds::rename_mod_folder,
            commands::mod_cmds::restore_mod,
            commands::mod_cmds::list_trash,
            commands::mod_cmds::empty_trash,
            commands::mod_cmds::read_mod_info,
            commands::mod_cmds::update_mod_info,
            commands::mod_cmds::pre_delete_check,
            commands::mod_cmds::set_mod_category,
            commands::mod_cmds::move_mod_to_object,
            commands::mod_cmds::update_mod_thumbnail,
            commands::mod_cmds::update_mod_thumbnail,
            commands::mod_cmds::paste_thumbnail,
            commands::folder_cmds::delete_mod_thumbnail,
            commands::mod_cmds::bulk_toggle_mods,
            commands::mod_cmds::bulk_delete_mods,
            commands::mod_cmds::bulk_update_info,
            commands::mod_cmds::import_mods_from_paths,
            commands::preview_cmds::list_mod_ini_files,
            commands::preview_cmds::read_mod_ini,
            commands::preview_cmds::write_mod_ini,
            commands::preview_cmds::list_mod_preview_images,
            commands::preview_cmds::save_mod_preview_image,
            commands::preview_cmds::remove_mod_preview_image,
            commands::preview_cmds::clear_mod_preview_images,
            commands::settings_cmds::get_settings,
            commands::settings_cmds::save_settings,
            commands::settings_cmds::set_safe_mode_pin,
            commands::settings_cmds::verify_pin,
            commands::settings_cmds::set_active_game,
            commands::settings_cmds::set_safe_mode_enabled,
            commands::settings_cmds::run_maintenance,
            commands::epic5_cmds::enable_only_this,
            commands::epic5_cmds::check_duplicate_enabled,
            commands::epic5_cmds::check_shader_conflicts,
            commands::collection_cmds::list_collections,
            commands::collection_cmds::create_collection,
            commands::collection_cmds::update_collection,
            commands::collection_cmds::delete_collection,
            commands::collection_cmds::apply_collection,
            commands::collection_cmds::undo_collection_apply,
            commands::collection_cmds::export_collection,
            commands::collection_cmds::import_collection,
            commands::scan_cmds::sync_database_cmd,
            commands::scan_cmds::scan_preview_cmd,
            commands::scan_cmds::commit_scan_cmd,
            commands::scan_cmds::start_watcher_cmd,
            commands::dup_scan_cmds::dup_scan_start,
            commands::dup_scan_cmds::dup_scan_cancel,
            commands::dup_scan_cmds::dup_scan_get_report,
            commands::dup_resolve_cmds::dup_resolve_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
