import os

types_dup_scan = "src-tauri/src/types/dup_scan.rs"
domain_dup_scan = "src-tauri/src/modules/duplicates/domain/dup_scan.rs"
mod_rs = "src-tauri/src/types/mod.rs"

if os.path.exists(types_dup_scan):
    with open(types_dup_scan, "r") as f:
        content = f.read()
    with open(domain_dup_scan, "w") as f:
        f.write(content)
        
    os.remove(types_dup_scan)
    if os.path.exists(mod_rs):
        os.remove(mod_rs)
        
    # Remove types from lib.rs
    with open("src-tauri/src/lib.rs", "r") as f:
        lib_content = f.read()
    lib_content = lib_content.replace("pub mod types;\n", "")
    with open("src-tauri/src/lib.rs", "w") as f:
        f.write(lib_content)
        
    os.rmdir("src-tauri/src/types")
    print("Moved dup_scan.rs to duplicates domain and deleted src-tauri/src/types")
