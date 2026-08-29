import os, shutil

base = "src"
mappings = {
    "library": ["commands/mods/conflict_cmds.rs", "commands/mods/mod_bulk_cmds.rs", "commands/mods/mod_core_cmds.rs", "commands/mods/mod_thumbnail_cmds.rs", "commands/mods/preview_cmds.rs", "commands/mods/trash_cmds.rs"],
    "catalog": ["commands/objects/master_db_cmds.rs"]
}

for module, files in mappings.items():
    mod_dir = os.path.join(base, "modules", module)
    inbound_dir = os.path.join(mod_dir, "adapters", "inbound")
    os.makedirs(inbound_dir, exist_ok=True)
            
    inbound_mod_content = ""
    # read existing mod.rs
    mod_rs_path = os.path.join(inbound_dir, "mod.rs")
    if os.path.exists(mod_rs_path):
        with open(mod_rs_path, "r") as f:
            inbound_mod_content = f.read()

    for file in files:
        src_file = os.path.join(base, file)
        if os.path.exists(src_file):
            filename = os.path.basename(file)
            mod_name = filename.replace(".rs", "")
            target_file = os.path.join(inbound_dir, filename)
            shutil.move(src_file, target_file)
            if f"pub mod {mod_name};" not in inbound_mod_content:
                inbound_mod_content += f"pub mod {mod_name};\n"
            
    with open(mod_rs_path, "w") as f:
        f.write(inbound_mod_content)

print("Done moving missed files")
