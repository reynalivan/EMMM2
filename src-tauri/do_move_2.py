import os
import shutil
import re

def mv(src, dest):
    if os.path.exists(src):
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.move(src, dest)
        return True
    return False

def remove_line(file_path, pattern):
    if not os.path.exists(file_path): return
    with open(file_path, "r") as f:
        lines = f.readlines()
    with open(file_path, "w") as f:
        for l in lines:
            if not re.search(pattern, l):
                f.write(l)

def write_mod(file_path, content):
    os.makedirs(os.path.dirname(file_path), exist_ok=True)
    with open(file_path, "w") as f:
        f.write(content)

# 1. settings
# We moved system/application/config -> settings/application/config
remove_line("src/modules/system/application/mod.rs", r"pub mod config;")

# 2. updates
mv("src/modules/system/application/update", "src/modules/updates/application/update")
remove_line("src/modules/system/application/mod.rs", r"pub mod update;")
write_mod("src/modules/updates/mod.rs", "pub mod application;\n")
write_mod("src/modules/updates/application/mod.rs", "pub mod update;\n")

# 3. matching
mv("src/modules/workspace/application/scanner/deep_matcher", "src/modules/matching/application/deep_matcher")
remove_line("src/modules/workspace/application/scanner/mod.rs", r"pub mod deep_matcher;")
write_mod("src/modules/matching/mod.rs", "pub mod application;\n")
write_mod("src/modules/matching/application/mod.rs", "pub mod deep_matcher;\n")

# 4. reconciliation
mv("src/modules/workspace/application/disk_reconcile", "src/modules/reconciliation/application/disk_reconcile")
# wait, there's inbound adapters for disk_reconcile_cmds. Let's move them too.
mv("src/modules/workspace/adapters/inbound/disk_reconcile_cmds.rs", "src/modules/reconciliation/adapters/inbound/disk_reconcile_cmds.rs")
remove_line("src/modules/workspace/application/mod.rs", r"pub mod disk_reconcile;")
remove_line("src/modules/workspace/adapters/inbound/mod.rs", r"pub mod disk_reconcile_cmds;")

write_mod("src/modules/reconciliation/mod.rs", "pub mod application;\npub mod adapters;\n")
write_mod("src/modules/reconciliation/application/mod.rs", "pub mod disk_reconcile;\n")
write_mod("src/modules/reconciliation/adapters/mod.rs", "pub mod inbound;\n")
write_mod("src/modules/reconciliation/adapters/inbound/mod.rs", "pub mod disk_reconcile_cmds;\n")

# 5. mutation
# We need app/runtime/mutation_coordinator.rs AND workspace_mutation
mv("src/app/runtime/mutation_coordinator.rs", "src/modules/mutation/coordinator.rs")
remove_line("src/app/runtime/mod.rs", r"pub mod mutation_coordinator;")
mv("src/modules/workspace/application/workspace_mutation", "src/modules/mutation/application/workspace_mutation")
remove_line("src/modules/workspace/application/mod.rs", r"pub mod workspace_mutation;")
write_mod("src/modules/mutation/mod.rs", "pub mod coordinator;\npub mod application;\n")
write_mod("src/modules/mutation/application/mod.rs", "pub mod workspace_mutation;\n")

# 6. duplicates
# rename storage_optimizer to duplicates
mv("src/modules/storage_optimizer", "src/modules/duplicates")
# also move workspace/application/scanner/dedup to duplicates/application/dedup
mv("src/modules/workspace/application/scanner/dedup", "src/modules/duplicates/application/dedup")
remove_line("src/modules/workspace/application/scanner/mod.rs", r"pub mod dedup;")
# add to duplicates/application/mod.rs (it already exists from storage_optimizer)
with open("src/modules/duplicates/application/mod.rs", "a") as f:
    f.write("pub mod dedup;\n")

# Fix root modules/mod.rs
# storage_optimizer -> duplicates
remove_line("src/modules/mod.rs", r"pub mod storage_optimizer;")
with open("src/modules/mod.rs", "a") as f:
    f.write("pub mod duplicates;\n")
    f.write("pub mod settings;\n")
    f.write("pub mod updates;\n")
    f.write("pub mod matching;\n")
    f.write("pub mod reconciliation;\n")
    f.write("pub mod mutation;\n")

print("Files moved and mod.rs files updated.")

