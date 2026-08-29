import os, re
filepath = "src/lib.rs"
with open(filepath, "r") as f:
    content = f.read()

mapping = {
    "commands::app::app_cmds": "crate::modules::system::adapters::inbound::app_cmds",
    "commands::app::settings_cmds": "crate::modules::system::adapters::inbound::settings_cmds",
    "commands::app::theme_cmds": "crate::modules::system::adapters::inbound::theme_cmds",
    "commands::app::update_cmds": "crate::modules::system::adapters::inbound::update_cmds",
    "commands::app::dashboard_cmds": "crate::modules::dashboard::adapters::inbound::dashboard_cmds",
    "commands::app::game_cmds": "crate::modules::games::adapters::inbound::game_cmds",
    "commands::app::hotkey_cmds": "crate::modules::automation::adapters::inbound::hotkey_cmds",
    "commands::app::workspace_cmds": "crate::modules::workspace::adapters::inbound::workspace_cmds",
    "commands::folder_grid::folder_grid_cmds": "crate::modules::workspace::adapters::inbound::folder_grid_cmds",
    "commands::folder_grid::folder_grid_state": "crate::modules::workspace::adapters::inbound::folder_grid_state",
    "commands::scanner::disk_reconcile_cmds": "crate::modules::workspace::adapters::inbound::disk_reconcile_cmds",
    "commands::scanner::folder_entries_cmds": "crate::modules::workspace::adapters::inbound::folder_entries_cmds",
    "commands::scanner::watcher_cmds": "crate::modules::workspace::adapters::inbound::watcher_cmds",
    "commands::folder_grid::folder_grid_mutations": "crate::modules::library::adapters::inbound::folder_grid_mutations",
    "commands::mods::mod_cmds": "crate::modules::library::adapters::inbound::mod_cmds",
    "commands::mods::mod_meta_cmds": "crate::modules::library::adapters::inbound::mod_meta_cmds",
    "commands::mods::thumbnail_cmds": "crate::modules::library::adapters::inbound::thumbnail_cmds",
    "commands::scanner::conflict_cmds": "crate::modules::library::adapters::inbound::conflict_cmds",
    "commands::mods::conflict_cmds": "crate::modules::library::adapters::inbound::conflict_cmds",
    "commands::mods::mod_bulk_cmds": "crate::modules::library::adapters::inbound::mod_bulk_cmds",
    "commands::mods::mod_core_cmds": "crate::modules::library::adapters::inbound::mod_core_cmds",
    "commands::mods::mod_thumbnail_cmds": "crate::modules::library::adapters::inbound::mod_thumbnail_cmds",
    "commands::mods::preview_cmds": "crate::modules::library::adapters::inbound::preview_cmds",
    "commands::mods::trash_cmds": "crate::modules::library::adapters::inbound::trash_cmds",
    "commands::objects::object_cmds": "crate::modules::catalog::adapters::inbound::object_cmds",
    "commands::objects::object_preview_cmds": "crate::modules::catalog::adapters::inbound::object_preview_cmds",
    "commands::objects::master_db_cmds": "crate::modules::catalog::adapters::inbound::master_db_cmds",
    "commands::folder_grid::get_mod_thumbnail": "crate::modules::library::adapters::inbound::thumbnail_cmds::get_mod_thumbnail", 
}

for old, new in mapping.items():
    content = content.replace(old, new)

with open(filepath, "w") as f:
    f.write(content)

print("lib.rs updated")
