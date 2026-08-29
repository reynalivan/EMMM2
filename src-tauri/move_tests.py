import os, shutil

base = "src"
# (module -> [list of test file names])
mappings = {
    "system": ["commands/app/tests/app_cmds_tests.rs", "commands/app/tests/settings_cmds_tests.rs", "commands/app/tests/theme_cmds_tests.rs", "commands/app/tests/update_cmds_tests.rs"],
    "dashboard": ["commands/app/tests/dashboard_cmds_tests.rs"],
    "games": ["commands/app/tests/game_cmds_tests.rs"],
    "automation": ["commands/app/tests/hotkey_cmds_tests.rs"],
    "workspace": ["commands/app/tests/workspace_cmds_tests.rs", "commands/folder_grid/tests/folder_grid_cmds_tests.rs", "commands/folder_grid/tests/folder_grid_state_tests.rs", "commands/scanner/tests/disk_reconcile_cmds_tests.rs", "commands/scanner/tests/folder_entries_cmds_tests.rs", "commands/scanner/tests/watcher_cmds_tests.rs"],
    "library": ["commands/folder_grid/tests/folder_grid_mutations_tests.rs", "commands/mods/tests/mod_cmds_tests.rs", "commands/mods/tests/mod_meta_cmds_tests.rs", "commands/mods/tests/thumbnail_cmds_tests.rs", "commands/mods/tests/conflict_cmds_tests.rs", "commands/mods/tests/mod_bulk_cmds_tests.rs", "commands/mods/tests/mod_core_cmds_tests.rs", "commands/mods/tests/mod_thumbnail_cmds_tests.rs", "commands/mods/tests/preview_cmds_tests.rs", "commands/mods/tests/trash_cmds_tests.rs", "commands/scanner/tests/conflict_cmds_tests.rs"],
    "catalog": ["commands/objects/tests/object_cmds_tests.rs", "commands/objects/tests/object_preview_cmds_tests.rs", "commands/objects/tests/master_db_cmds_tests.rs"]
}

for module, files in mappings.items():
    mod_dir = os.path.join(base, "modules", module)
    inbound_dir = os.path.join(mod_dir, "adapters", "inbound")
    tests_dir = os.path.join(inbound_dir, "tests")
    os.makedirs(tests_dir, exist_ok=True)
            
    for file in files:
        src_file = os.path.join(base, file)
        if os.path.exists(src_file):
            filename = os.path.basename(file)
            target_file = os.path.join(tests_dir, filename)
            shutil.move(src_file, target_file)

print("Done moving test files")
