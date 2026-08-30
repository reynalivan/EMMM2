import os
import shutil
import glob

def mv(src, dest):
    if os.path.exists(src):
        print(f"Moving {src} -> {dest}")
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.move(src, dest)

def ensure_mod(dir_path):
    os.makedirs(dir_path, exist_ok=True)
    mod_path = os.path.join(dir_path, "mod.rs")
    if not os.path.exists(mod_path):
        with open(mod_path, "w") as f:
            f.write("// generated\n")

# 1. settings
mv("src/modules/system/application/config", "src/modules/settings/application")
ensure_mod("src/modules/settings")
with open("src/modules/settings/mod.rs", "w") as f:
    f.write("pub mod application;\n")
with open("src/modules/settings/application/mod.rs", "w") as f:
    f.write("pub mod config;\n")

# Wait, it's better to just move `config` as the whole module content or keep it in application?
# "settings" is the module. Let's make `settings/` have what `config/` had.
# Actually, the user spec says:
# modules/settings/
# ├── domain
# ├── application
# ├── adapters
# Let's just move `config` to `settings/application/config` for now to avoid refactoring the internals too much.
