import os, shutil

base = "src"

def setup_module(mod_name):
    mod_dir = os.path.join(base, "modules", mod_name)
    os.makedirs(os.path.join(mod_dir, "application"), exist_ok=True)
    os.makedirs(os.path.join(mod_dir, "adapters/outbound/sqlite"), exist_ok=True)
    os.makedirs(os.path.join(mod_dir, "domain"), exist_ok=True)
    return mod_dir

def move_dir(src, dest):
    if os.path.exists(src):
        os.makedirs(dest, exist_ok=True)
        for f in os.listdir(src):
            shutil.move(os.path.join(src, f), os.path.join(dest, f))
        os.rmdir(src)

def move_file(src, dest):
    if os.path.exists(src):
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.move(src, dest)

# 1. Dashboard
mod_dash = setup_module("dashboard")
move_dir(os.path.join(base, "repo/dashboard"), os.path.join(mod_dash, "adapters/outbound/sqlite/dashboard"))
move_file(os.path.join(base, "domain/dashboard.rs"), os.path.join(mod_dash, "domain/dashboard.rs"))

with open(os.path.join(mod_dash, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod dashboard;\n")
with open(os.path.join(mod_dash, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_dash, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_dash, "domain/mod.rs"), "w") as f:
    f.write("pub mod dashboard;\n")
with open(os.path.join(mod_dash, "mod.rs"), "a") as f:
    f.write("pub mod domain;\n")

# 2. Games
mod_games = setup_module("games")
move_dir(os.path.join(base, "services/game"), os.path.join(mod_games, "application/game"))
move_dir(os.path.join(base, "repo/game"), os.path.join(mod_games, "adapters/outbound/sqlite/game"))
move_file(os.path.join(base, "domain/models.rs"), os.path.join(mod_games, "domain/models.rs"))

with open(os.path.join(mod_games, "application/mod.rs"), "w") as f:
    f.write("pub mod game;\n")
with open(os.path.join(mod_games, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod game;\n")
with open(os.path.join(mod_games, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_games, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_games, "domain/mod.rs"), "w") as f:
    f.write("pub mod models;\n")
with open(os.path.join(mod_games, "mod.rs"), "a") as f:
    f.write("pub mod application;\npub mod domain;\n")

# 3. Automation
mod_auto = setup_module("automation")
move_dir(os.path.join(base, "services/hotkeys"), os.path.join(mod_auto, "application/hotkeys"))
move_dir(os.path.join(base, "services/keyviewer"), os.path.join(mod_auto, "application/keyviewer"))

with open(os.path.join(mod_auto, "application/mod.rs"), "w") as f:
    f.write("pub mod hotkeys;\npub mod keyviewer;\n")
with open(os.path.join(mod_auto, "mod.rs"), "a") as f:
    f.write("pub mod application;\n")

# 4. Browser
mod_browser = setup_module("browser")
move_dir(os.path.join(base, "services/browser"), os.path.join(mod_browser, "application/browser"))
move_dir(os.path.join(base, "repo/browser"), os.path.join(mod_browser, "adapters/outbound/sqlite/browser"))
move_file(os.path.join(base, "domain/browser.rs"), os.path.join(mod_browser, "domain/browser.rs"))

with open(os.path.join(mod_browser, "application/mod.rs"), "w") as f:
    f.write("pub mod browser;\n")
with open(os.path.join(mod_browser, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod browser;\n")
with open(os.path.join(mod_browser, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_browser, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_browser, "domain/mod.rs"), "w") as f:
    f.write("pub mod browser;\n")
with open(os.path.join(mod_browser, "mod.rs"), "a") as f:
    f.write("pub mod application;\npub mod domain;\n")

print("Moved dashboard, games, automation, browser")
