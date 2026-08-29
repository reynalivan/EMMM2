import os, shutil

base = "src"

# Target directories
plat_dir = os.path.join(base, "platform")
shared_dir = os.path.join(base, "shared")

os.makedirs(os.path.join(plat_dir, "fs"), exist_ok=True)
os.makedirs(os.path.join(plat_dir, "images"), exist_ok=True)
os.makedirs(shared_dir, exist_ok=True)

# 1. fs_utils
src_fs = os.path.join(base, "services/fs_utils")
if os.path.exists(src_fs):
    for f in os.listdir(src_fs):
        shutil.move(os.path.join(src_fs, f), os.path.join(plat_dir, "fs", f))
    os.rmdir(src_fs)

# 2. images
src_img = os.path.join(base, "services/images")
if os.path.exists(src_img):
    for f in os.listdir(src_img):
        shutil.move(os.path.join(src_img, f), os.path.join(plat_dir, "images", f))
    os.rmdir(src_img)

# 3. shared files
src_err = os.path.join(base, "domain/errors.rs")
if os.path.exists(src_err):
    shutil.move(src_err, os.path.join(shared_dir, "errors.rs"))

src_path = os.path.join(base, "common/path_key.rs")
if os.path.exists(src_path):
    shutil.move(src_path, os.path.join(shared_dir, "path_key.rs"))

src_sync = os.path.join(base, "common/sync.rs")
if os.path.exists(src_sync):
    shutil.move(src_sync, os.path.join(shared_dir, "sync.rs"))

# Create mod.rs files
with open(os.path.join(plat_dir, "mod.rs"), "w") as f:
    f.write("pub mod fs;\npub mod images;\n")

with open(os.path.join(shared_dir, "mod.rs"), "w") as f:
    f.write("pub mod errors;\npub mod path_key;\npub mod sync;\n")

print("Moved Platform and Shared files")
