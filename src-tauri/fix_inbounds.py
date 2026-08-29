import os

for mod in ["ingestion", "catalog", "library"]:
    filepath = f"src/modules/{mod}/adapters/mod.rs"
    with open(filepath, "r") as f:
        content = f.read()
    if "pub mod inbound;" not in content:
        with open(filepath, "a") as f:
            f.write("pub mod inbound;\n")

# Also fix the other missing imports in post_apply.rs and bootstrap.rs
import re
def rep(fp, old, new):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace(old, new)
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/system/application/app/post_apply.rs", "repo::object", "crate::modules::catalog::adapters::outbound::sqlite::object")
rep("src/modules/system/application/app/bootstrap.rs", "repo::import_batch", "crate::modules::ingestion::adapters::outbound::sqlite::import_batch")
rep("src/modules/system/application/app/bootstrap.rs", "services::import_batch", "crate::modules::ingestion::application::import_batch")

print("Fixed adapters/mod.rs and some remaining imports")
