import os

def rep(fp, old, new):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace(old, new)
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/workspace/domain/tests/classifier_tests.rs", "crate::common::classifier", "crate::modules::workspace::domain::classifier")
rep("src/modules/storage_optimizer/adapters/outbound/sqlite/dedup/tests.rs", "crate::repo::game", "crate::modules::games::adapters::outbound::sqlite::game")
rep("src/modules/storage_optimizer/adapters/outbound/sqlite/dedup/tests.rs", "crate::domain::models", "crate::modules::games::domain::models")

print("Fixed test imports")
