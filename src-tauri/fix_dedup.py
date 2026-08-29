import os

def rep(fp):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace("crate::repo::dedup", "crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup")
    c = c.replace("repo::dedup", "crate::modules::storage_optimizer::adapters::outbound::sqlite::dedup")
    c = c.replace("pub mod common;\n", "")
    c = c.replace("pub mod domain;\n", "")
    c = c.replace("pub mod repo;\n", "")
    c = c.replace("pub mod services;\n", "")
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/workspace/application/scanner/dedup/scanner.rs")
rep("src/lib.rs")
print("Fixed dedup imports and lib.rs")
