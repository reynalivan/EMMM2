import os

files = {
    "src-tauri/src/app/runtime/operation_journal.rs": "src-tauri/src/modules/mutation/journal.rs",
    "src-tauri/src/app/runtime/recovery_runner.rs": "src-tauri/src/modules/mutation/recovery.rs",
    "src-tauri/src/app/runtime/task_registry.rs": "src-tauri/src/modules/mutation/task_registry.rs",
}

for src, dst in files.items():
    if os.path.exists(src):
        with open(src, "r") as f:
            content = f.read()
        with open(dst, "w") as f:
            f.write(content)

os.remove("src-tauri/src/app/runtime/operation_journal.rs")
os.remove("src-tauri/src/app/runtime/recovery_runner.rs")
os.remove("src-tauri/src/app/runtime/task_registry.rs")
os.remove("src-tauri/src/app/runtime/mod.rs")
os.rmdir("src-tauri/src/app/runtime")
os.remove("src-tauri/src/app/mod.rs")
os.rmdir("src-tauri/src/app")

# Fix lib.rs to remove pub mod app;
with open("src-tauri/src/lib.rs", "r") as f:
    lib_content = f.read()
lib_content = lib_content.replace("pub mod app;\n", "")
with open("src-tauri/src/lib.rs", "w") as f:
    f.write(lib_content)

print("Moved app/runtime files to mutation and removed app/")
