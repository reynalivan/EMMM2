import subprocess

def get_lines_old(paths):
    lines = 0
    for path in paths:
        res = subprocess.run(["git", "ls-tree", "-r", "33b2180", path], capture_output=True, text=True)
        for line in res.stdout.strip().split('\n'):
            if not line: continue
            filepath = line.split('\t')[1]
            out = subprocess.run(["git", "show", f"33b2180:{filepath}"], capture_output=True, text=True)
            lines += len(out.stdout.split('\n'))
    return lines

def get_lines_new(grep_strs):
    res = subprocess.run(["git", "ls-files"], capture_output=True, text=True)
    lines = 0
    for filepath in res.stdout.strip().split('\n'):
        if not filepath: continue
        if any(g in filepath for g in grep_strs):
            try:
                with open(filepath, 'r', encoding='utf-8') as f:
                    lines += len(f.readlines())
            except Exception:
                pass
    return lines

features = [
    ("collection", ["src-tauri/src/services/collection", "src-tauri/src/repo/collection", "src-tauri/src/commands/collections"], ["src/modules/collections"]),
    ("enable disabled", ["src-tauri/src/services/mods/status.rs", "src-tauri/src/services/mods/toggle.rs"], ["src/modules/library/application/mods/status", "src/modules/library/application/mods/toggle"]),
    ("filewatcher", ["src-tauri/src/services/scanner/watcher"], ["src/modules/workspace/application/scanner/watcher"]),
    ("safe unsafe (privacy)", ["src-tauri/src/services/workspace/privacy"], ["src/modules/privacy"]),
    ("auto classification", ["src-tauri/src/services/objects/classification", "src-tauri/src/services/imports/classification.rs"], ["src/modules/catalog/application/objects/classif"]),
    ("ini read", ["src-tauri/src/services/mods/ini.rs", "src-tauri/src/services/mods/metadata.rs"], ["src/modules/library/adapters/outbound/sqlite/ini", "src/modules/library/application/mods/metadata"]),
    ("preview", ["src-tauri/src/services/mods/preview_ops.rs"], ["src/modules/library/application/mods/preview_ops.rs"]),
    ("objectlist (catalog)", ["src-tauri/src/services/objects", "src-tauri/src/repo/objects"], ["src/modules/catalog"]),
    ("duplicate scanner", ["src-tauri/src/services/scanner/dedup", "src-tauri/src/repo/dedup"], ["src/modules/storage_optimizer"]),
    ("browser", ["src-tauri/src/services/browser", "src-tauri/src/commands/browser"], ["src/modules/browser"]),
]

print("--- Comparison of Feature Line Counts (33b2180 vs HEAD) ---")
for name, old, new in features:
    old_c = get_lines_old(old)
    new_c = get_lines_new(new)
    print(f"{name:25} : Old={old_c:5} lines -> New={new_c:5} lines")

