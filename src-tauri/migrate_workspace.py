import os, shutil

base = "src"
mod_workspace = os.path.join(base, "modules", "workspace")

os.makedirs(os.path.join(mod_workspace, "application"), exist_ok=True)
os.makedirs(os.path.join(mod_workspace, "adapters/outbound/sqlite"), exist_ok=True)
os.makedirs(os.path.join(mod_workspace, "domain"), exist_ok=True)

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

# application
move_dir(os.path.join(base, "services/disk_reconcile"), os.path.join(mod_workspace, "application/disk_reconcile"))
move_dir(os.path.join(base, "services/explorer"), os.path.join(mod_workspace, "application/explorer"))
move_dir(os.path.join(base, "services/workspace"), os.path.join(mod_workspace, "application/workspace"))
move_dir(os.path.join(base, "services/workspace_mutation"), os.path.join(mod_workspace, "application/workspace_mutation"))
move_dir(os.path.join(base, "services/workspace_read_model"), os.path.join(mod_workspace, "application/workspace_read_model"))
move_dir(os.path.join(base, "services/projected_state"), os.path.join(mod_workspace, "application/projected_state"))
move_dir(os.path.join(base, "services/recovery"), os.path.join(mod_workspace, "application/recovery"))
move_dir(os.path.join(base, "services/scanner"), os.path.join(mod_workspace, "application/scanner"))

# outbound sqlite
move_dir(os.path.join(base, "repo/runtime_projection"), os.path.join(mod_workspace, "adapters/outbound/sqlite/runtime_projection"))
move_dir(os.path.join(base, "repo/conflict"), os.path.join(mod_workspace, "adapters/outbound/sqlite/conflict"))
move_dir(os.path.join(base, "repo/task"), os.path.join(mod_workspace, "adapters/outbound/sqlite/task"))

# domain
move_dir(os.path.join(base, "domain/workspace"), os.path.join(mod_workspace, "domain/workspace"))
move_file(os.path.join(base, "domain/runtime_state.rs"), os.path.join(mod_workspace, "domain/runtime_state.rs"))
move_file(os.path.join(base, "domain/conflicts.rs"), os.path.join(mod_workspace, "domain/conflicts.rs"))
move_file(os.path.join(base, "domain/task.rs"), os.path.join(mod_workspace, "domain/task.rs"))
move_file(os.path.join(base, "common/classifier.rs"), os.path.join(mod_workspace, "domain/classifier.rs"))
move_file(os.path.join(base, "common/normalizer.rs"), os.path.join(mod_workspace, "domain/normalizer.rs"))

# Create mods
with open(os.path.join(mod_workspace, "application/mod.rs"), "w") as f:
    f.write("pub mod disk_reconcile;\npub mod explorer;\npub mod workspace;\npub mod workspace_mutation;\npub mod workspace_read_model;\npub mod projected_state;\npub mod recovery;\npub mod scanner;\n")

with open(os.path.join(mod_workspace, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod runtime_projection;\npub mod conflict;\npub mod task;\n")

with open(os.path.join(mod_workspace, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")

with open(os.path.join(mod_workspace, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")

with open(os.path.join(mod_workspace, "domain/mod.rs"), "w") as f:
    f.write("pub mod workspace;\npub mod runtime_state;\npub mod conflicts;\npub mod task;\npub mod classifier;\npub mod normalizer;\n")

with open(os.path.join(mod_workspace, "mod.rs"), "a") as f:
    f.write("pub mod application;\npub mod domain;\n")

print("Moved workspace module")
