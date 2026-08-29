import os, shutil

base = "src"
mod_sys = os.path.join(base, "modules/system")

# Target directories
app_dir = os.path.join(mod_sys, "application")
out_sqlite = os.path.join(mod_sys, "adapters/outbound/sqlite")
dom_dir = os.path.join(mod_sys, "domain")

os.makedirs(os.path.join(app_dir, "app"), exist_ok=True)
os.makedirs(os.path.join(app_dir, "config"), exist_ok=True)
os.makedirs(os.path.join(app_dir, "update"), exist_ok=True)
os.makedirs(os.path.join(out_sqlite, "settings"), exist_ok=True)
os.makedirs(os.path.join(out_sqlite, "utils"), exist_ok=True)
os.makedirs(dom_dir, exist_ok=True)

def move_dir(src, dest):
    if os.path.exists(src):
        for f in os.listdir(src):
            shutil.move(os.path.join(src, f), os.path.join(dest, f))
        os.rmdir(src)

move_dir(os.path.join(base, "services/app"), os.path.join(app_dir, "app"))
move_dir(os.path.join(base, "services/config"), os.path.join(app_dir, "config"))
move_dir(os.path.join(base, "services/update"), os.path.join(app_dir, "update"))

move_dir(os.path.join(base, "repo/settings"), os.path.join(out_sqlite, "settings"))
move_dir(os.path.join(base, "repo/utils"), os.path.join(out_sqlite, "utils"))

src_dom = os.path.join(base, "domain/mod_path.rs")
if os.path.exists(src_dom):
    shutil.move(src_dom, os.path.join(dom_dir, "mod_path.rs"))

# Mod declarations
with open(os.path.join(app_dir, "mod.rs"), "w") as f:
    f.write("pub mod app;\npub mod config;\npub mod update;\n")

# Outbound
outbound_mod = os.path.join(mod_sys, "adapters/outbound/mod.rs")
os.makedirs(os.path.dirname(outbound_mod), exist_ok=True)
with open(outbound_mod, "w") as f:
    f.write("pub mod sqlite;\n")

# Sqlite mod
with open(os.path.join(out_sqlite, "mod.rs"), "w") as f:
    f.write("pub mod settings;\npub mod utils;\n")

# Adapters
adapters_mod = os.path.join(mod_sys, "adapters/mod.rs")
with open(adapters_mod, "r") as f:
    content = f.read()
if "pub mod outbound;" not in content:
    with open(adapters_mod, "a") as f:
        f.write("pub mod outbound;\n")

with open(os.path.join(dom_dir, "mod.rs"), "w") as f:
    f.write("pub mod mod_path;\n")

print("Moved System files")
