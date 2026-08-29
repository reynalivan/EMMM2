import os, re
filepath = "src/lib.rs"
with open(filepath, "r") as f:
    content = f.read()

mapping = {
    "crate::modules::library::adapters::inbound::conflict_cmds::detect_conflicts_cmd": "crate::modules::workspace::adapters::inbound::scanner_conflict_cmds::detect_conflicts_cmd",
    "crate::modules::library::adapters::inbound::conflict_cmds::detect_conflicts_in_folder_cmd": "crate::modules::workspace::adapters::inbound::scanner_conflict_cmds::detect_conflicts_in_folder_cmd",
    "commands::folder_grid::delete_mod_thumbnail": "crate::modules::library::adapters::inbound::thumbnail_cmds::delete_mod_thumbnail"
}

for old, new in mapping.items():
    content = content.replace(old, new)

with open(filepath, "w") as f:
    f.write(content)

print("lib.rs fixed")
