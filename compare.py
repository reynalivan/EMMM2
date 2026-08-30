import subprocess

def get_lines_old(path):
    # git ls-tree -r 33b2180 path
    res = subprocess.run(["git", "ls-tree", "-r", "33b2180", path], capture_output=True, text=True)
    lines = 0
    for line in res.stdout.strip().split('\n'):
        if not line: continue
        filepath = line.split('\t')[1]
        out = subprocess.run(["git", "show", f"33b2180:{filepath}"], capture_output=True, text=True)
        lines += len(out.stdout.split('\n'))
    return lines

def get_lines_new(grep_str):
    # git ls-files | grep grep_str
    res = subprocess.run(["git", "ls-files"], capture_output=True, text=True)
    lines = 0
    for filepath in res.stdout.strip().split('\n'):
        if not filepath: continue
        if grep_str in filepath:
            try:
                with open(filepath, 'r', encoding='utf-8') as f:
                    lines += len(f.readlines())
            except Exception:
                pass
    return lines

features = [
    ("collection", "src-tauri/src/services/collections", "src/modules/collections"),
    ("enable disabled", "src-tauri/src/services/mods/status.rs", "src/modules/library/application/mods/status.rs"),
    ("filewatcher", "src-tauri/src/services/scanner/watcher", "src/modules/workspace/application/scanner/watcher"),
    ("privacy (safe)", "src-tauri/src/services/workspace_mutation/privacy", "src/modules/privacy"),
    ("classification", "src-tauri/src/services/objects/classification", "src/modules/catalog/application/objects/classif"),
    ("ini handling", "src-tauri/src/services/mods/ini.rs", "src/modules/library/application/mods/ini"),
    ("preview", "src-tauri/src/services/mods/preview_ops.rs", "src/modules/library/application/mods/preview_ops.rs"),
    ("objects (catalog)", "src-tauri/src/services/objects", "src/modules/catalog/application/objects"),
    ("duplicate scanner", "src-tauri/src/services/scanner/dedup", "src/modules/workspace/application/scanner/dedup"),
    ("browser", "src-tauri/src/services/browser", "src/modules/browser"),
]

for name, old, new in features:
    old_c = get_lines_old(old)
    new_c = get_lines_new(new)
    print(f"{name}: Old={old_c} lines | New={new_c} lines")

