import os, shutil

base = "src"

# Move safety_constants
if os.path.exists(os.path.join(base, "common/safety_constants.rs")):
    shutil.move(os.path.join(base, "common/safety_constants.rs"), os.path.join(base, "shared/safety_constants.rs"))
    os.rmdir(os.path.join(base, "common"))

with open(os.path.join(base, "shared/mod.rs"), "a") as f:
    f.write("pub mod safety_constants;\n")

# Replace in files
def rep_in_all(old, new):
    for root, dirs, files in os.walk(base):
        for file in files:
            if file.endswith(".rs"):
                fp = os.path.join(root, file)
                with open(fp, "r", encoding="utf-8") as f:
                    c = f.read()
                if old in c:
                    c = c.replace(old, new)
                    with open(fp, "w", encoding="utf-8") as f:
                        f.write(c)

rep_in_all("crate::common::safety_constants", "crate::shared::safety_constants")
rep_in_all("crate::domain::conflicts", "crate::modules::workspace::domain::conflicts")
rep_in_all("crate::domain::errors", "crate::shared::errors")
rep_in_all("crate::repo::dedup", "crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup")

# Clean bootstrap and post_apply
def remove_lines(fp, lines_to_remove):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        lines = f.readlines()
    new_lines = [l for l in lines if l.strip() not in lines_to_remove]
    with open(fp, "w", encoding="utf-8") as f:
        f.writelines(new_lines)

remove_lines("src/modules/system/application/app/bootstrap.rs", ["use crate::repo;", "use crate::services;"])
remove_lines("src/modules/system/application/app/post_apply.rs", ["use crate::repo;"])
remove_lines("src/modules/storage_optimizer/adapters/mod.rs", ["pub mod outbound;"])
with open("src/modules/storage_optimizer/adapters/mod.rs", "a") as f:
    f.write("pub mod outbound;\n")

print("Fixed forgotten things")
