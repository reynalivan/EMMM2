#[cfg(not(test))]
use tauri::Manager;
#[cfg(not(test))]
use tauri_plugin_log::{Target, TargetKind};


pub mod app;
pub mod modules;
pub mod platform;
pub mod shared;
pub mod pipeline;
#[cfg(test)]
pub mod test_utils;
pub mod types;

/// Standard prefix for disabled mod folders. Shared across commands.
pub const DISABLED_PREFIX: &str = "DISABLED ";

macro_rules! emmm_collect_commands {
    () => {
        tauri_specta::collect_commands![
            crate::modules::system::adapters::inbound::app_cmds::check_config_status,
            crate::modules::dashboard::adapters::inbound::dashboard_cmds::get_dashboard_stats,
            crate::modules::dashboard::adapters::inbound::dashboard_cmds::get_active_keybindings,
            crate::modules::workspace::adapters::inbound::workspace_cmds::get_workspace_view_model,
            crate::modules::workspace::adapters::inbound::workspace_cmds::execute_workspace_switch,
            crate::modules::system::adapters::inbound::app_cmds::get_logs,
            crate::modules::system::adapters::inbound::app_cmds::open_log_folder,
            crate::modules::system::adapters::inbound::app_cmds::reset_database,
            crate::modules::system::adapters::inbound::app_cmds::check_path_exists_cmd,
            crate::modules::games::adapters::inbound::game_cmds::auto_detect_games,
            crate::modules::games::adapters::inbound::game_cmds::resolve_game_folder,
            crate::modules::games::adapters::inbound::game_cmds::add_game_manual,
            crate::modules::games::adapters::inbound::game_cmds::save_onboarding_games,
            crate::modules::games::adapters::inbound::game_cmds::get_games,
            crate::modules::games::adapters::inbound::game_cmds::launch_game,
            crate::modules::catalog::adapters::inbound::master_db_cmds::get_game_schema,
            crate::modules::catalog::adapters::inbound::master_db_cmds::get_object,
            crate::modules::catalog::adapters::inbound::master_db_cmds::get_master_db,
            crate::modules::catalog::adapters::inbound::master_db_cmds::search_master_db,
            crate::modules::catalog::adapters::inbound::master_db_cmds::pin_object,
            crate::modules::workspace::adapters::inbound::scanner_conflict_cmds::detect_conflicts_cmd,
            crate::modules::workspace::adapters::inbound::scanner_conflict_cmds::detect_conflicts_in_folder_cmd,
            crate::modules::library::adapters::inbound::thumbnail_cmds::get_mod_thumbnail,
            crate::modules::library::adapters::inbound::mod_core_cmds::open_in_explorer,
            crate::modules::library::adapters::inbound::mod_core_cmds::reveal_object_in_explorer,
            crate::modules::library::adapters::inbound::conflict_cmds::get_folder_conflict_details,
            crate::modules::library::adapters::inbound::conflict_cmds::resolve_folder_name_conflict,
            crate::modules::library::adapters::inbound::conflict_cmds::trash_folder_conflict_candidate,
            crate::modules::library::adapters::inbound::conflict_cmds::ignore_object_conflict,
            crate::modules::library::adapters::inbound::conflict_cmds::revoke_object_conflict,
            crate::modules::library::adapters::inbound::conflict_cmds::list_ignored_object_conflicts,
            crate::modules::library::adapters::inbound::mod_core_cmds::rename_mod_folder,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_toggle_mods,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_delete_mods,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_update_info,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_set_mod_safety,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_toggle_favorite,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_pin_mods,
            crate::modules::library::adapters::inbound::mod_bulk_cmds::bulk_cancel,
            crate::modules::library::adapters::inbound::mod_meta_cmds::toggle_mod_safe,
            crate::modules::library::adapters::inbound::mod_meta_cmds::suggest_random_mods,
            crate::modules::library::adapters::inbound::mod_meta_cmds::get_active_mod_conflicts,
            crate::modules::library::adapters::inbound::mod_meta_cmds::read_mod_info,
            crate::modules::library::adapters::inbound::mod_meta_cmds::update_mod_info,
            crate::modules::library::adapters::inbound::mod_meta_cmds::set_mod_category,
            crate::modules::library::adapters::inbound::mod_meta_cmds::set_object_mods_category,
            crate::modules::library::adapters::inbound::mod_meta_cmds::list_move_targets_for_object,
            crate::modules::library::adapters::inbound::mod_meta_cmds::move_mods_to_object,
            crate::modules::library::adapters::inbound::mod_thumbnail_cmds::update_mod_thumbnail,
            crate::modules::library::adapters::inbound::mod_thumbnail_cmds::paste_thumbnail,
            crate::modules::library::adapters::inbound::thumbnail_cmds::delete_mod_thumbnail,
            crate::modules::library::adapters::inbound::trash_cmds::delete_mod,
            crate::modules::library::adapters::inbound::trash_cmds::open_recycle_bin,
            crate::modules::library::adapters::inbound::preview_cmds::list_mod_ini_files,
            crate::modules::library::adapters::inbound::preview_cmds::read_mod_ini,
            crate::modules::library::adapters::inbound::preview_cmds::write_mod_ini,
            crate::modules::library::adapters::inbound::preview_cmds::list_mod_preview_images,
            crate::modules::library::adapters::inbound::preview_cmds::save_mod_preview_image,
            crate::modules::library::adapters::inbound::preview_cmds::remove_mod_preview_image,
            crate::modules::library::adapters::inbound::preview_cmds::clear_mod_preview_images,
            modules::ingestion::adapters::inbound::tauri::create_import_batch,
            modules::ingestion::adapters::inbound::tauri::get_import_batch,
            modules::ingestion::adapters::inbound::tauri::list_import_batches,
            modules::ingestion::adapters::inbound::tauri::analyze_import_batch,
            modules::ingestion::adapters::inbound::tauri::set_import_item_classification,
            modules::ingestion::adapters::inbound::tauri::refresh_import_item_suggestions,
            modules::ingestion::adapters::inbound::tauri::set_import_item_decision,
            modules::ingestion::adapters::inbound::tauri::rename_import_item_plan,
            modules::ingestion::adapters::inbound::tauri::cancel_import_batch,
            modules::ingestion::adapters::inbound::tauri::commit_import_batch,
            modules::ingestion::adapters::inbound::tauri::get_mod_inbox,
            modules::ingestion::adapters::inbound::tauri::create_mod_inbox_folder,
            modules::ingestion::adapters::inbound::tauri::open_mod_inbox_folder,
            modules::ingestion::adapters::inbound::tauri::create_mod_inbox_batch,
            modules::ingestion::adapters::inbound::tauri::delete_processed_mod_inbox_sources,
            modules::ingestion::adapters::inbound::tauri::start_mod_inbox_watcher,
            modules::ingestion::adapters::inbound::tauri::stop_mod_inbox_watcher,
            modules::ingestion::adapters::inbound::tauri::preview_object_classification_batch,
            modules::ingestion::adapters::inbound::tauri::apply_object_classification_batch,
            modules::ingestion::adapters::inbound::tauri::preview_relocation_batch,
            crate::modules::system::adapters::inbound::settings_cmds::get_settings,
            crate::modules::system::adapters::inbound::settings_cmds::save_settings,
            crate::modules::system::adapters::inbound::settings_cmds::set_active_game,
            crate::modules::system::adapters::inbound::settings_cmds::set_auto_close_launcher,
            crate::modules::system::adapters::inbound::settings_cmds::run_maintenance,
            crate::modules::system::adapters::inbound::settings_cmds::clear_old_thumbnails,
            crate::modules::system::adapters::inbound::theme_cmds::list_custom_themes,
            crate::modules::system::adapters::inbound::theme_cmds::load_custom_theme,
            crate::modules::system::adapters::inbound::theme_cmds::save_custom_theme,
            crate::modules::system::adapters::inbound::theme_cmds::delete_custom_theme,
            crate::modules::catalog::adapters::inbound::object_cmds::get_objects_cmd,
            crate::modules::catalog::adapters::inbound::object_cmds::get_category_counts_cmd,
            crate::modules::catalog::adapters::inbound::object_cmds::create_object_cmd,
            crate::modules::catalog::adapters::inbound::object_cmds::update_object_cmd,
            crate::modules::catalog::adapters::inbound::object_cmds::delete_object_cmd,
            modules::collections::adapters::inbound::tauri::get_collection_runtime_state,
            modules::collections::adapters::inbound::tauri::get_collection_runtime_descriptor,
            modules::collections::adapters::inbound::tauri::get_apply_progress,
            modules::collections::adapters::inbound::tauri::list_collections,
            modules::collections::adapters::inbound::tauri::create_collection,
            modules::collections::adapters::inbound::tauri::save_current_runtime_as_collection,
            modules::collections::adapters::inbound::tauri::apply_collection,
            modules::collections::adapters::inbound::tauri::update_collection,
            modules::collections::adapters::inbound::tauri::replace_collection_with_current_state,
            modules::collections::adapters::inbound::tauri::save_collection_changes,
            modules::collections::adapters::inbound::tauri::restore_last_changes,
            modules::collections::adapters::inbound::tauri::clear_last_changes,
            modules::collections::adapters::inbound::tauri::delete_collection,
            modules::collections::adapters::inbound::tauri::app_startup_check,
            modules::collections::adapters::inbound::tauri::resolve_recovery_task,
            modules::collections::adapters::inbound::tauri::get_collection_preview,
            modules::collections::adapters::inbound::tauri::preview_apply_collection,
            crate::modules::workspace::adapters::inbound::folder_entries_cmds::list_folder_entries_cmd,
            crate::modules::workspace::adapters::inbound::disk_reconcile_cmds::apply_game_mods_directory,
            crate::modules::workspace::adapters::inbound::disk_reconcile_cmds::reconcile_disk_state_cmd,
            crate::modules::workspace::adapters::inbound::disk_reconcile_cmds::inspect_game_mods_directory,
            crate::modules::workspace::adapters::inbound::disk_reconcile_cmds::resolve_rename_confirmations,
            crate::modules::workspace::adapters::inbound::watcher_cmds::start_watcher,
            crate::modules::workspace::adapters::inbound::watcher_cmds::stop_watcher,
            modules::storage_optimizer::adapters::inbound::tauri::dup_scan_start,
            modules::storage_optimizer::adapters::inbound::tauri::dup_scan_cancel,
            modules::storage_optimizer::adapters::inbound::tauri::dup_scan_get_report,
            modules::storage_optimizer::adapters::inbound::tauri::dup_resolve_batch,
            modules::storage_optimizer::adapters::inbound::tauri::get_ignored_pairs,
            modules::storage_optimizer::adapters::inbound::tauri::remove_ignored_pair,
            crate::modules::system::adapters::inbound::update_cmds::check_metadata_update,
            crate::modules::system::adapters::inbound::update_cmds::fetch_missing_asset,
            crate::modules::automation::adapters::inbound::hotkey_cmds::update_hotkey_config,
            crate::modules::automation::adapters::inbound::hotkey_cmds::get_reload_key,
            modules::browser::adapters::inbound::tauri::browser_open_tab,
            modules::browser::adapters::inbound::tauri::browser_navigate,
            modules::browser::adapters::inbound::tauri::browser_go_back,
            modules::browser::adapters::inbound::tauri::browser_go_forward,
            modules::browser::adapters::inbound::tauri::browser_reload_tab,
            modules::browser::adapters::inbound::tauri::browser_clear_data,
            modules::browser::adapters::inbound::tauri::browser_get_homepage,
            modules::browser::adapters::inbound::tauri::browser_set_homepage,
            modules::browser::adapters::inbound::tauri::browser_list_downloads,
            modules::browser::adapters::inbound::tauri::browser_cancel_download,
            modules::browser::adapters::inbound::tauri::browser_delete_download,
            modules::browser::adapters::inbound::tauri::browser_clear_imported,
            modules::browser::adapters::inbound::tauri::browser_clear_old_downloads,
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
                        app.try_state::<crate::modules::automation::application::hotkeys::manager::HotkeyManager>()
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
        .manage(crate::modules::workspace::application::scanner::watcher::WatcherState::new())
        .manage(crate::modules::ingestion::application::import_batch::mod_inbox_watcher::ModInboxWatcherState::new())
        .manage(crate::modules::workspace::application::disk_reconcile::orchestrator::DiskReconcileState::new())
        .setup(move |app| {
            let app_handle = app.handle();

            crate::modules::system::application::app::bootstrap::center_window_if_offscreen(app_handle);

            #[cfg(desktop)]
            app_handle.plugin(tauri_plugin_updater::Builder::new().build())?;

            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                crate::platform::images::thumbnail_cache::ThumbnailCache::init(&app_data_dir);

                #[cfg(desktop)]
                app.manage(crate::modules::system::application::app::bootstrap::init_pool(&app_data_dir));
            }

            let pool_ref: tauri::State<'_, sqlx::SqlitePool> = app.state();
            app.manage(crate::modules::system::application::config::ConfigService::init(
                app_handle,
                pool_ref.inner().clone(),
            ));

            let config_ref: tauri::State<'_, crate::modules::system::application::config::ConfigService> = app.state();
            let hotkey_config = config_ref.get_settings().hotkeys;
            app.manage(crate::modules::system::application::app::bootstrap::init_hotkey_manager(
                app_handle,
                &hotkey_config,
            ));

            {
                crate::modules::system::application::app::bootstrap::run_startup_reconcile(app.handle().clone());
            }

            Ok(())
        })
        .manage(modules::storage_optimizer::DupScanState::new())
        .manage(crate::modules::library::adapters::inbound::mod_bulk_cmds::BulkCancelState::new())
        .manage(crate::platform::fs::operation_lock::OperationLock::new())
        .manage(crate::app::runtime::operation_journal::OperationJournal::new())
        .manage(crate::app::runtime::mutation_coordinator::MutationCoordinator::new(std::sync::Arc::new(crate::app::runtime::operation_journal::OperationJournal::new())))

        .manage(crate::modules::workspace::application::scanner::master_db::MasterDbCache::default())
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
        let command_pattern =
            regex::Regex::new(r"(?:commands|modules)(?:::[A-Za-z0-9_]+)+::([A-Za-z0-9_]+),")
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
    /// definitions. CI runs `git diff --exit-code src/shared/api/tauri/bindings.gen.ts` after
    /// `cargo test`, so any Rust payload change that is not committed to the
    /// generated file fails the build (type-drift guard).
    #[test]
    fn export_bindings() {
        let output_path = std::path::Path::new("..").join("src/shared/api/tauri/bindings.gen.ts");
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
