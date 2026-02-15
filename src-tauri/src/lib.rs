use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_sql::{Migration, MigrationKind};

pub mod commands;
pub mod database;
pub mod services;
#[cfg(test)]
pub mod test_utils;

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
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:app.db", migrations)
                .build(),
        )
        .manage(commands::scan_cmds::ScanState::new())
        .manage(services::watcher::WatcherState::new())
        .invoke_handler(tauri::generate_handler![
            commands::app_cmds::check_config_status,
            commands::game_cmds::auto_detect_games,
            commands::game_cmds::add_game_manual,
            commands::game_cmds::get_games,
            commands::object_cmds::get_game_schema,
            commands::scan_cmds::cancel_scan_cmd,
            commands::scan_cmds::detect_archives_cmd,
            commands::scan_cmds::extract_archive_cmd,
            commands::scan_cmds::analyze_archive_cmd,
            commands::scan_cmds::start_scan,
            commands::scan_cmds::get_scan_result,
            commands::scan_cmds::detect_conflicts_cmd,
            commands::scan_cmds::detect_conflicts_in_folder_cmd,
            commands::scan_cmds::set_watcher_suppression_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
