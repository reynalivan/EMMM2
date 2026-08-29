#[cfg(not(test))]
use tauri::Manager;
#[cfg(not(test))]
use tauri_plugin_log::{Target, TargetKind};

pub mod commands;
pub mod common;
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
            commands::app::dashboard_cmds::get_dashboard_stats,
            commands::app::dashboard_cmds::get_active_keybindings,
            commands::app::workspace_cmds::get_workspace_view_model,
            commands::app::workspace_cmds::execute_workspace_switch,
            commands::app::app_cmds::get_logs,
            commands::app::app_cmds::open_log_folder,
            commands::app::app_cmds::reset_database,
            commands::app::app_cmds::check_path_exists_cmd,
            commands::app::game_cmds::auto_detect_games,
            commands::app::game_cmds::resolve_game_folder,
            commands::app::game_cmds::add_game_manual,
            commands::app::game_cmds::save_onboarding_games,
            commands::app::game_cmds::get_games,
            commands::app::game_cmds::launch_game,
            commands::objects::master_db_cmds::get_game_schema,
            commands::objects::master_db_cmds::get_object,
            commands::objects::master_db_cmds::get_master_db,
            commands::objects::master_db_cmds::search_master_db,
            commands::objects::master_db_cmds::pin_object,
            commands::scanner::conflict_cmds::detect_conflicts_cmd,
            commands::scanner::conflict_cmds::detect_conflicts_in_folder_cmd,
            commands::folder_grid::get_mod_thumbnail,
            commands::mods::mod_core_cmds::open_in_explorer,
            commands::mods::mod_core_cmds::reveal_object_in_explorer,
            commands::mods::conflict_cmds::get_folder_conflict_details,
            commands::mods::conflict_cmds::resolve_folder_name_conflict,
            commands::mods::conflict_cmds::trash_folder_conflict_candidate,
            commands::mods::conflict_cmds::ignore_object_conflict,
            commands::mods::conflict_cmds::revoke_object_conflict,
            commands::mods::conflict_cmds::list_ignored_object_conflicts,
            commands::mods::mod_core_cmds::rename_mod_folder,
            commands::mods::mod_bulk_cmds::bulk_toggle_mods,
            commands::mods::mod_bulk_cmds::bulk_delete_mods,
            commands::mods::mod_bulk_cmds::bulk_update_info,
            commands::mods::mod_bulk_cmds::bulk_set_mod_safety,
            commands::mods::mod_bulk_cmds::bulk_toggle_favorite,
            commands::mods::mod_bulk_cmds::bulk_pin_mods,
            commands::mods::mod_bulk_cmds::bulk_cancel,
            commands::mods::mod_meta_cmds::toggle_mod_safe,
            commands::mods::mod_meta_cmds::suggest_random_mods,
            commands::mods::mod_meta_cmds::get_active_mod_conflicts,
            commands::mods::mod_meta_cmds::read_mod_info,
            commands::mods::mod_meta_cmds::update_mod_info,
            commands::mods::mod_meta_cmds::set_mod_category,
            commands::mods::mod_meta_cmds::set_object_mods_category,
            commands::mods::mod_meta_cmds::list_move_targets_for_object,
            commands::mods::mod_meta_cmds::move_mods_to_object,
            commands::mods::mod_thumbnail_cmds::update_mod_thumbnail,
            commands::mods::mod_thumbnail_cmds::paste_thumbnail,
            commands::folder_grid::delete_mod_thumbnail,
            commands::mods::trash_cmds::delete_mod,
            commands::mods::trash_cmds::open_recycle_bin,
            commands::mods::preview_cmds::list_mod_ini_files,
            commands::mods::preview_cmds::read_mod_ini,
            commands::mods::preview_cmds::write_mod_ini,
            commands::mods::preview_cmds::list_mod_preview_images,
            commands::mods::preview_cmds::save_mod_preview_image,
            commands::mods::preview_cmds::remove_mod_preview_image,
            commands::mods::preview_cmds::clear_mod_preview_images,
            commands::imports::import_batch_cmds::create_import_batch,
            commands::imports::import_batch_cmds::get_import_batch,
            commands::imports::import_batch_cmds::list_import_batches,
            commands::imports::import_batch_cmds::analyze_import_batch,
            commands::imports::import_batch_cmds::set_import_item_classification,
            commands::imports::import_batch_cmds::refresh_import_item_suggestions,
            commands::imports::import_batch_cmds::set_import_item_decision,
            commands::imports::import_batch_cmds::rename_import_item_plan,
            commands::imports::import_batch_cmds::cancel_import_batch,
            commands::imports::import_batch_cmds::commit_import_batch,
            commands::imports::import_batch_cmds::get_mod_inbox,
            commands::imports::import_batch_cmds::create_mod_inbox_folder,
            commands::imports::import_batch_cmds::open_mod_inbox_folder,
            commands::imports::import_batch_cmds::create_mod_inbox_batch,
            commands::imports::import_batch_cmds::delete_processed_mod_inbox_sources,
            commands::imports::import_batch_cmds::start_mod_inbox_watcher,
            commands::imports::import_batch_cmds::stop_mod_inbox_watcher,
            commands::imports::classification_cmds::preview_object_classification_batch,
            commands::imports::classification_cmds::apply_object_classification_batch,
            commands::imports::classification_cmds::preview_relocation_batch,
            commands::app::settings_cmds::get_settings,
            commands::app::settings_cmds::save_settings,
            commands::app::settings_cmds::set_active_game,
            commands::app::settings_cmds::set_auto_close_launcher,
            commands::app::settings_cmds::run_maintenance,
            commands::app::settings_cmds::clear_old_thumbnails,
            commands::app::theme_cmds::list_custom_themes,
            commands::app::theme_cmds::load_custom_theme,
            commands::app::theme_cmds::save_custom_theme,
            commands::app::theme_cmds::delete_custom_theme,
            commands::objects::object_cmds::get_objects_cmd,
            commands::objects::object_cmds::get_category_counts_cmd,
            commands::objects::object_cmds::create_object_cmd,
            commands::objects::object_cmds::update_object_cmd,
            commands::objects::object_cmds::delete_object_cmd,
            commands::collections::cmds::get_collection_runtime_state,
            commands::collections::cmds::get_collection_runtime_descriptor,
            commands::collections::cmds::get_apply_progress,
            commands::collections::cmds::list_collections,
            commands::collections::cmds::create_collection,
            commands::collections::cmds::save_current_runtime_as_collection,
            commands::collections::cmds::apply_collection,
            commands::collections::cmds::update_collection,
            commands::collections::cmds::replace_collection_with_current_state,
            commands::collections::cmds::save_collection_changes,
            commands::collections::cmds::restore_last_changes,
            commands::collections::cmds::clear_last_changes,
            commands::collections::cmds::delete_collection,
            commands::collections::cmds::app_startup_check,
            commands::collections::cmds::resolve_recovery_task,
            commands::collections::cmds::get_collection_preview,
            commands::collections::cmds::preview_apply_collection,
            commands::scanner::folder_entries_cmds::list_folder_entries_cmd,
            commands::scanner::disk_reconcile_cmds::apply_game_mods_directory,
            commands::scanner::disk_reconcile_cmds::reconcile_disk_state_cmd,
            commands::scanner::disk_reconcile_cmds::inspect_game_mods_directory,
            commands::scanner::disk_reconcile_cmds::resolve_rename_confirmations,
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
            commands::app::hotkey_cmds::get_reload_key,
            commands::browser::browser_cmds::browser_open_tab,
            commands::browser::browser_cmds::browser_navigate,
            commands::browser::browser_cmds::browser_go_back,
            commands::browser::browser_cmds::browser_go_forward,
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
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[cfg(not(test))]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new().commands(emmm_collect_commands!());

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
                    if let Some(hotkey_manager) =
                        app.try_state::<services::hotkeys::manager::HotkeyManager>()
                    {
                        hotkey_manager
                            .inner()
                            .on_shortcut_pressed(app, &shortcut.to_string());
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
        .manage(services::import_batch::mod_inbox_watcher::ModInboxWatcherState::new())
        .manage(services::disk_reconcile::orchestrator::DiskReconcileState::new())
        .setup(move |app| {
            let app_handle = app.handle();

            services::app::bootstrap::center_window_if_offscreen(app_handle);

            #[cfg(desktop)]
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;

            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                services::images::thumbnail_cache::ThumbnailCache::init(&app_data_dir);

                #[cfg(desktop)]
                app.manage(services::app::bootstrap::init_pool(&app_data_dir));
            }

            let pool_ref: tauri::State<'_, sqlx::SqlitePool> = app.state();
            app.manage(services::config::ConfigService::init(
                app_handle,
                pool_ref.inner().clone(),
            ));

            let config_ref: tauri::State<'_, services::config::ConfigService> = app.state();
            let hotkey_config = config_ref.get_settings().hotkeys;
            app.manage(services::app::bootstrap::init_hotkey_manager(
                app_handle,
                &hotkey_config,
            ));

            {
                services::app::bootstrap::run_startup_reconcile(app.handle().clone());
            }

            Ok(())
        })
        .manage(commands::duplicates::dup_scan_cmds::DupScanState::new())
        .manage(commands::mods::mod_bulk_cmds::BulkCancelState::new())
        .manage(services::fs_utils::operation_lock::OperationLock::new())
        .manage(services::scanner::master_db::MasterDbCache::default())
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
    fn every_registered_command_is_allowed_by_the_app_permission() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let command_source = std::fs::read_to_string(manifest_dir.join("src/lib.rs"))
            .expect("read command registry");
        let permission_source =
            std::fs::read_to_string(manifest_dir.join("permissions/app-commands.toml"))
                .expect("read app command permission");
        let command_pattern = regex::Regex::new(r"commands(?:::[A-Za-z0-9_]+)+::([A-Za-z0-9_]+),")
            .expect("valid command regex");
        let allowed_pattern =
            regex::Regex::new(r#"\"([A-Za-z0-9_]+)\""#).expect("valid allowlist regex");

        let registered = command_pattern
            .captures_iter(&command_source)
            .map(|capture| capture[1].to_string())
            .collect::<std::collections::BTreeSet<_>>();
        let allowed = allowed_pattern
            .captures_iter(&permission_source)
            .map(|capture| capture[1].to_string())
            .collect::<std::collections::BTreeSet<_>>();
        let missing = registered.difference(&allowed).cloned().collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "commands missing from permissions/app-commands.toml (runtime `command not allowed`): {}",
            missing.join(", ")
        );
    }

    /// Regenerates the committed frontend type bindings from the Rust command/type
    /// definitions. CI runs `git diff --exit-code src/lib/bindings.gen.ts` after
    /// `cargo test`, so any Rust payload change that is not committed to the
    /// generated file fails the build (type-drift guard).
    #[test]
    fn export_bindings() {
        let output_path = std::path::Path::new("..").join("src/core/tauri/bindings.gen.ts");
        tauri_specta::Builder::<tauri::Wry>::new()
            .commands(emmm_collect_commands!())
            .export(
                specta_typescript::Typescript::default()
                    .header("// @ts-nocheck\n/* eslint-disable */")
                    .bigint(specta_typescript::BigIntExportBehavior::Number),
                &output_path,
            )
            .expect("The types could not be exported");
        let generated = std::fs::read_to_string(&output_path).expect("read generated bindings");
        let normalized = generated
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(output_path, normalized).expect("normalize generated bindings");
    }
}
