import os

base = "src"
mapping = {
    "crate::services::fs_utils": "crate::platform::fs",
    "crate::services::images": "crate::platform::images",
    "crate::domain::errors": "crate::shared::errors",
    "crate::common::path_key": "crate::shared::path_key",
    "crate::common::sync": "crate::shared::sync"
}

for root, dirs, files in os.walk(base):
    for f in files:
        if f.endswith(".rs"):
            filepath = os.path.join(root, f)
            with open(filepath, "r", encoding="utf-8") as file:
                content = file.read()
                
            original = content
            for old, new in mapping.items():
                content = content.replace(old, new)
                
            if content != original:
                with open(filepath, "w", encoding="utf-8") as file:
                    file.write(content)

# Update lib.rs to include platform and shared
with open(os.path.join(base, "lib.rs"), "r", encoding="utf-8") as f:
    lib_content = f.read()

if "pub mod platform;" not in lib_content:
    lib_content = lib_content.replace("pub mod modules;", "pub mod modules;\npub mod platform;\npub mod shared;")
    with open(os.path.join(base, "lib.rs"), "w", encoding="utf-8") as f:
        f.write(lib_content)

# Remove legacy exports
def remove_line(filepath, line):
    if not os.path.exists(filepath): return
    with open(filepath, "r") as f:
        content = f.read()
    content = content.replace(line + "\n", "")
    with open(filepath, "w") as f:
        f.write(content)

remove_line("src/services/mod.rs", "pub mod fs_utils;")
remove_line("src/services/mod.rs", "pub mod images;")
remove_line("src/domain/mod.rs", "pub mod errors;")
remove_line("src/common/mod.rs", "pub mod path_key;")
remove_line("src/common/mod.rs", "pub mod sync;")

print("Imports updated")
