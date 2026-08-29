import os, shutil

base = "src"
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
    
    with open(os.path.join(mod_dir, "adapters", "mod.rs"), "w") as f:
        f.write("pub mod inbound;\n")
        
    mod_file = os.path.join(mod_dir, "mod.rs")
    mod_content = ""
    if os.path.exists(mod_file):
        with open(mod_file, "r") as f:
            mod_content = f.read()
    if "mod adapters;" not in mod_content and "pub mod adapters;" not in mod_content:
        with open(mod_file, "a") as f:
            f.write("\npub mod adapters;\n")
            
    inbound_mod_content = ""
    for file in files:
        src_file = os.path.join(base, file)
        if os.path.exists(src_file):
            filename = os.path.basename(file)
            mod_name = filename.replace(".rs", "")
            target_file = os.path.join(inbound_dir, filename)
            shutil.move(src_file, target_file)
            inbound_mod_content += f"pub mod {mod_name};\n"
            
    with open(os.path.join(inbound_dir, "mod.rs"), "w") as f:
        f.write(inbound_mod_content)

print("Done moving files")
