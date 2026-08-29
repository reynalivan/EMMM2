import os, shutil

base = "src"

# Move models_test
shutil.move(os.path.join(base, "domain/models_test.rs"), os.path.join(base, "modules/games/domain/models_test.rs"))
os.rmdir(os.path.join(base, "domain"))

# Storage Optimizer setup
mod_storage = os.path.join(base, "modules/storage_optimizer")
os.makedirs(os.path.join(mod_storage, "adapters/outbound/sqlite"), exist_ok=True)
shutil.move(os.path.join(base, "repo/dedup"), os.path.join(mod_storage, "adapters/outbound/sqlite/dedup"))
os.rmdir(os.path.join(base, "repo"))

with open(os.path.join(mod_storage, "adapters/outbound/sqlite/mod.rs"), "w") as f:
    f.write("pub mod dedup;\n")
with open(os.path.join(mod_storage, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_storage, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")

print("Restored and migrated storage_optimizer and models_test")
