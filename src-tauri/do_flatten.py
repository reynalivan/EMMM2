import os
import shutil

def mv(src, dest):
    if os.path.exists(src):
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.move(src, dest)
        return True
    return False

# We iterate over all modules in src/modules
modules_dir = "src/modules"
if os.path.exists(modules_dir):
    for mod in os.listdir(modules_dir):
        mod_path = os.path.join(modules_dir, mod)
        if os.path.isdir(mod_path):
            adapters_dir = os.path.join(mod_path, "adapters")
            if os.path.exists(adapters_dir):
                # Move inbound -> tauri
                inbound_dir = os.path.join(adapters_dir, "inbound")
                if os.path.exists(inbound_dir):
                    mv(inbound_dir, os.path.join(adapters_dir, "tauri"))
                    
                # Move outbound subdirs
                outbound_dir = os.path.join(adapters_dir, "outbound")
                if os.path.exists(outbound_dir):
                    for sub in os.listdir(outbound_dir):
                        sub_path = os.path.join(outbound_dir, sub)
                        if os.path.isdir(sub_path):
                            mv(sub_path, os.path.join(adapters_dir, sub))
                        elif os.path.isfile(sub_path):
                            # if it's a file like repo.rs
                            mv(sub_path, os.path.join(adapters_dir, sub))
                            
                    # Remove outbound if empty
                    if not os.listdir(outbound_dir):
                        os.rmdir(outbound_dir)
                        
                # Update adapters/mod.rs
                adapters_mod = os.path.join(adapters_dir, "mod.rs")
                if os.path.exists(adapters_mod):
                    with open(adapters_mod, "r") as f:
                        lines = f.readlines()
                    with open(adapters_mod, "w") as f:
                        for line in lines:
                            line = line.replace("pub mod inbound;", "pub mod tauri;")
                            line = line.replace("pub mod outbound;", "")
                            if "pub mod sqlite;" not in line and os.path.exists(os.path.join(adapters_dir, "sqlite")):
                                # we just write it safely below
                                pass
                            f.write(line)
                            
                    # Ensure new dirs are in mod.rs
                    with open(adapters_mod, "r") as f:
                        content = f.read()
                    with open(adapters_mod, "a") as f:
                        if os.path.exists(os.path.join(adapters_dir, "sqlite")) and "pub mod sqlite;" not in content:
                            f.write("pub mod sqlite;\n")
                        if os.path.exists(os.path.join(adapters_dir, "ini")) and "pub mod ini;" not in content:
                            f.write("pub mod ini;\n")

# Global replace in all rs files
replacements = {
    "adapters::inbound::": "adapters::tauri::",
    "adapters::outbound::sqlite::": "adapters::sqlite::",
    "adapters::outbound::ini::": "adapters::ini::",
    "adapters::outbound::": "adapters::",
}

def replace_in_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            
        modified = False
        for old, new in replacements.items():
            if old in content:
                content = content.replace(old, new)
                modified = True
                
        if modified:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
    except Exception as e:
        pass

for root, _, files in os.walk("."):
    for file in files:
        if file.endswith(".rs"):
            replace_in_file(os.path.join(root, file))

print("Flattened adapters and updated references.")
