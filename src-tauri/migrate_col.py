import os, shutil

base = "src"
mod_col = os.path.join(base, "modules/collections")

# Target directories
app_col = os.path.join(mod_col, "application/collection")
app_runtime = os.path.join(mod_col, "application/runtime")
out_sqlite = os.path.join(mod_col, "adapters/outbound/sqlite")
dom_dir = os.path.join(mod_col, "domain")

os.makedirs(app_col, exist_ok=True)
os.makedirs(app_runtime, exist_ok=True)
os.makedirs(out_sqlite, exist_ok=True)
os.makedirs(dom_dir, exist_ok=True)

# 1. Move services/collection/* -> application/collection/*
src_svc_col = os.path.join(base, "services/collection")
if os.path.exists(src_svc_col):
    for f in os.listdir(src_svc_col):
        shutil.move(os.path.join(src_svc_col, f), os.path.join(app_col, f))
    os.rmdir(src_svc_col)

# 2. Move services/collection_runtime/* -> application/runtime/*
src_svc_rt = os.path.join(base, "services/collection_runtime")
if os.path.exists(src_svc_rt):
    for f in os.listdir(src_svc_rt):
        shutil.move(os.path.join(src_svc_rt, f), os.path.join(app_runtime, f))
    os.rmdir(src_svc_rt)

# 3. Move repo/collection/* -> adapters/outbound/sqlite/*
src_repo_col = os.path.join(base, "repo/collection")
if os.path.exists(src_repo_col):
    for f in os.listdir(src_repo_col):
        shutil.move(os.path.join(src_repo_col, f), os.path.join(out_sqlite, f))
    os.rmdir(src_repo_col)

# 4. Move domain/collection.rs -> domain/collection.rs
src_dom = os.path.join(base, "domain/collection.rs")
if os.path.exists(src_dom):
    shutil.move(src_dom, os.path.join(dom_dir, "collection.rs"))

# 5. Fix module declarations
with open(os.path.join(mod_col, "application/mod.rs"), "w") as f:
    f.write("pub mod collection;\npub mod runtime;\n")
with open(os.path.join(mod_col, "adapters/outbound/mod.rs"), "w") as f:
    f.write("pub mod sqlite;\n")
with open(os.path.join(mod_col, "adapters/mod.rs"), "a") as f:
    f.write("pub mod outbound;\n")
with open(os.path.join(mod_col, "domain/mod.rs"), "w") as f:
    f.write("pub mod collection;\n")
    
print("Moved files for collections module")
