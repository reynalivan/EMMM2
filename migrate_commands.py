import os, shutil

base = "src-tauri/src"
mappings = {
    "system": ["commands/app/app_cmds.rs", "commands/app/settings_cmds.rs", "commands/app/theme_cmds.rs", "commands/app/update_cmds.rs"],
    "dashboard": ["commands/app/dashboard_cmds.rs"],
    "games": ["commands/app/game_cmds.rs"],
    "automation": ["commands/app/hotkey_cmds.rs"],
    "workspace": ["commands/app/workspace_cmds.rs", "commands/folder_grid/folder_grid_cmds.rs", "commands/folder_grid/folder_grid_state.rs", "commands/scanner/disk_reconcile_cmds.rs", "commands/scanner/folder_entries_cmds.rs", "commands/scanner/watcher_cmds.rs"],
    "library": ["commands/folder_grid/folder_grid_mutations.rs", "commands/mods/mod_cmds.rs", "commands/mods/mod_meta_cmds.rs", "commands/mods/thumbnail_cmds.rs", "commands/scanner/conflict_cmds.rs"],
    "catalog": ["commands/objects/object_cmds.rs", "commands/objects/object_preview_cmds.rs"]
}

for module, files in mappings.items():
    mod_dir = os.path.join(base, "modules", module)
    inbound_dir = os.path.join(mod_dir, "adapters", "inbound")
    os.makedirs(inbound_dir, exist_ok=True)
    
    # write mod.rs for adapters and inbound
    with open(os.path.join(mod_dir, "adapters", "mod.rs"), "w") as f:
        f.write("pub mod inbound;\n")
    with open(os.path.join(inbound_dir, "mod.rs"), "w") as f:
        f.write("pub mod tauri;\n")
    
    # update mod_dir/mod.rs if not exists or doesn't have adapters
    mod_file = os.path.join(mod_dir, "mod.rs")
    mod_content = ""
    if os.path.exists(mod_file):
        with open(mod_file, "r") as f:
            mod_content = f.read()
    if "mod adapters;" not in mod_content and "pub mod adapters;" not in mod_content:
        with open(mod_file, "a") as f:
            f.write("\npub mod adapters;\n")
            
    # merge commands into tauri.rs
    tauri_file = os.path.join(inbound_dir, "tauri.rs")
    with open(tauri_file, "w") as out:
        # some imports might conflict if we just concat, but let's just concat for now.
        # usually they have their own use statements inside. 
        # wait, if they have `#![allow(...)]` at the top, they might fail if not at the beginning.
        # we will strip `#![...]` and put it at the top if needed.
        for file in files:
            src_file = os.path.join(base, file)
            if os.path.exists(src_file):
                with open(src_file, "r") as f:
                    out.write(f"\n// --- From {file} ---\n")
                    out.write(f.read())
                    out.write("\n")
                    
print("Done migrating files")
