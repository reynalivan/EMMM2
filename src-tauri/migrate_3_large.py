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

# 1. Ingestion
mod_ingestion = setup_module("ingestion")
move_dir(os.path.join(base, "services/import_batch"), os.path.join(mod_ingestion, "application/import_batch"))
move_dir(os.path.join(base, "repo/import_batch"), os.path.join(mod_ingestion, "adapters/outbound/sqlite/import_batch"))

with open(os.path.join(mod_ingestion, "application/mod.rs"), "w") as f:
    f.write("pub mod import_batch;\n")
with open(os.path.join(mod_ingestion, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod import_batch;\n")
with open(os.path.join(mod_ingestion, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_ingestion, "adapters/mod.rs"), "w") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_ingestion, "mod.rs"), "w") as f:
    f.write("pub mod adapters;\npub mod application;\n")

# 2. Catalog
mod_catalog = setup_module("catalog")
move_dir(os.path.join(base, "services/match_engine"), os.path.join(mod_catalog, "application/match_engine"))
move_dir(os.path.join(base, "services/objects"), os.path.join(mod_catalog, "application/objects"))
move_dir(os.path.join(base, "repo/object"), os.path.join(mod_catalog, "adapters/outbound/sqlite/object"))
move_file(os.path.join(base, "domain/objects.rs"), os.path.join(mod_catalog, "domain/objects.rs"))

with open(os.path.join(mod_catalog, "application/mod.rs"), "w") as f:
    f.write("pub mod match_engine;\npub mod objects;\n")
with open(os.path.join(mod_catalog, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod object;\n")
with open(os.path.join(mod_catalog, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_catalog, "adapters/mod.rs"), "w") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_catalog, "domain/mod.rs"), "w") as f:
    f.write("pub mod objects;\n")
with open(os.path.join(mod_catalog, "mod.rs"), "w") as f:
    f.write("pub mod adapters;\npub mod application;\npub mod domain;\n")

# 3. Library
mod_library = setup_module("library")
move_dir(os.path.join(base, "services/mods"), os.path.join(mod_library, "application/mods"))
move_dir(os.path.join(base, "services/ini"), os.path.join(mod_library, "application/ini"))
move_dir(os.path.join(base, "services/apply_progress"), os.path.join(mod_library, "application/apply_progress"))
move_dir(os.path.join(base, "repo/mods"), os.path.join(mod_library, "adapters/outbound/sqlite/mods"))
move_file(os.path.join(base, "domain/mods.rs"), os.path.join(mod_library, "domain/mods.rs"))

with open(os.path.join(mod_library, "application/mod.rs"), "w") as f:
    f.write("pub mod mods;\npub mod ini;\npub mod apply_progress;\n")
with open(os.path.join(mod_library, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod mods;\n")
with open(os.path.join(mod_library, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_library, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_library, "domain/mod.rs"), "w") as f:
    f.write("pub mod mods;\n")
with open(os.path.join(mod_library, "mod.rs"), "a") as f:
    f.write("pub mod application;\npub mod domain;\n")

print("Moved ingestion, catalog, library")
