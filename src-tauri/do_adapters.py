import os
import shutil
import re

def mv(src, dest):
    if os.path.exists(src):
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.move(src, dest)

def write_mod(file_path, content):
    os.makedirs(os.path.dirname(file_path), exist_ok=True)
    with open(file_path, "w") as f:
        f.write(content)
        
def remove_line(file_path, pattern):
    if not os.path.exists(file_path): return
    with open(file_path, "r") as f:
        lines = f.readlines()
    with open(file_path, "w") as f:
        for l in lines:
            if not re.search(pattern, l):
                f.write(l)

def replace_in_file(filepath, replacements):
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

mv("src/modules/system/adapters/inbound/settings_cmds.rs", "src/modules/settings/adapters/inbound/settings_cmds.rs")
mv("src/modules/system/adapters/inbound/update_cmds.rs", "src/modules/updates/adapters/inbound/update_cmds.rs")
mv("src/modules/system/adapters/inbound/tests/update_cmds_tests.rs", "src/modules/updates/adapters/inbound/tests/update_cmds_tests.rs")

remove_line("src/modules/system/adapters/inbound/mod.rs", r"pub mod settings_cmds;")
remove_line("src/modules/system/adapters/inbound/mod.rs", r"pub mod update_cmds;")

# Append to settings/mod.rs
with open("src/modules/settings/mod.rs", "a") as f:
    f.write("pub mod adapters;\n")
write_mod("src/modules/settings/adapters/mod.rs", "pub mod inbound;\n")
write_mod("src/modules/settings/adapters/inbound/mod.rs", "pub mod settings_cmds;\n")

# Append to updates/mod.rs
with open("src/modules/updates/mod.rs", "a") as f:
    f.write("pub mod adapters;\n")
write_mod("src/modules/updates/adapters/mod.rs", "pub mod inbound;\n")
write_mod("src/modules/updates/adapters/inbound/mod.rs", "pub mod update_cmds;\n")

# Replace in lib.rs
replacements = {
    "crate::modules::system::adapters::inbound::settings_cmds": "crate::modules::settings::adapters::inbound::settings_cmds",
    "crate::modules::system::adapters::inbound::update_cmds": "crate::modules::updates::adapters::inbound::update_cmds"
}
replace_in_file("src/lib.rs", replacements)

print("Moved inbound adapters for settings and updates.")
