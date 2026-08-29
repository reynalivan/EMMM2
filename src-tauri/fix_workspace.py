import os

mapping = {
    # Services -> Application
    "crate::services::disk_reconcile": "crate::modules::workspace::application::disk_reconcile",
    "crate::services::explorer": "crate::modules::workspace::application::explorer",
    "crate::services::workspace": "crate::modules::workspace::application::workspace",
    "crate::services::workspace_mutation": "crate::modules::workspace::application::workspace_mutation",
    "crate::services::workspace_read_model": "crate::modules::workspace::application::workspace_read_model",
    "crate::services::projected_state": "crate::modules::workspace::application::projected_state",
    "crate::services::recovery": "crate::modules::workspace::application::recovery",
    "crate::services::scanner": "crate::modules::workspace::application::scanner",
    
    "emmm_lib::services::disk_reconcile": "emmm_lib::modules::workspace::application::disk_reconcile",
    "emmm_lib::services::explorer": "emmm_lib::modules::workspace::application::explorer",
    "emmm_lib::services::workspace": "emmm_lib::modules::workspace::application::workspace",
    "emmm_lib::services::workspace_mutation": "emmm_lib::modules::workspace::application::workspace_mutation",
    "emmm_lib::services::workspace_read_model": "emmm_lib::modules::workspace::application::workspace_read_model",
    "emmm_lib::services::projected_state": "emmm_lib::modules::workspace::application::projected_state",
    "emmm_lib::services::recovery": "emmm_lib::modules::workspace::application::recovery",
    "emmm_lib::services::scanner": "emmm_lib::modules::workspace::application::scanner",
    
    # Repo -> Adapters
    "crate::repo::runtime_projection": "crate::modules::workspace::adapters::outbound::sqlite::runtime_projection",
    "crate::repo::conflict": "crate::modules::workspace::adapters::outbound::sqlite::conflict",
    "crate::repo::task": "crate::modules::workspace::adapters::outbound::sqlite::task",
    
    "emmm_lib::repo::runtime_projection": "emmm_lib::modules::workspace::adapters::outbound::sqlite::runtime_projection",
    "emmm_lib::repo::conflict": "emmm_lib::modules::workspace::adapters::outbound::sqlite::conflict",
    "emmm_lib::repo::task": "emmm_lib::modules::workspace::adapters::outbound::sqlite::task",
    
    # Domain -> Domain
    "crate::domain::workspace": "crate::modules::workspace::domain::workspace",
    "crate::domain::runtime_state": "crate::modules::workspace::domain::runtime_state",
    "crate::domain::conflicts": "crate::modules::workspace::domain::conflicts",
    "crate::domain::task": "crate::modules::workspace::domain::task",
    "crate::common::classifier": "crate::modules::workspace::domain::classifier",
    "crate::common::normalizer": "crate::modules::workspace::domain::normalizer",
    
    "emmm_lib::domain::workspace": "emmm_lib::modules::workspace::domain::workspace",
    "emmm_lib::domain::runtime_state": "emmm_lib::modules::workspace::domain::runtime_state",
    "emmm_lib::domain::conflicts": "emmm_lib::modules::workspace::domain::conflicts",
    "emmm_lib::domain::task": "emmm_lib::modules::workspace::domain::task",
    "emmm_lib::common::classifier": "emmm_lib::modules::workspace::domain::classifier",
    "emmm_lib::common::normalizer": "emmm_lib::modules::workspace::domain::normalizer",
}

for base in ["src", "tests"]:
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

def remove_line(filepath, line):
    if not os.path.exists(filepath): return
    with open(filepath, "r") as f:
        content = f.read()
    content = content.replace(line + "\n", "")
    with open(filepath, "w") as f:
        f.write(content)

remove_line("src/repo/mod.rs", "pub mod runtime_projection;")
remove_line("src/repo/mod.rs", "pub mod conflict;")
remove_line("src/repo/mod.rs", "pub mod task;")

remove_line("src/domain/mod.rs", "pub mod workspace;")
remove_line("src/domain/mod.rs", "pub mod runtime_state;")
remove_line("src/domain/mod.rs", "pub mod conflicts;")
remove_line("src/domain/mod.rs", "pub mod task;")

remove_line("src/common/mod.rs", "pub mod classifier;")
remove_line("src/common/mod.rs", "pub mod normalizer;")

remove_line("src/services/mod.rs", "pub mod disk_reconcile;")
remove_line("src/services/mod.rs", "pub mod explorer;")
remove_line("src/services/mod.rs", "pub mod workspace;")
remove_line("src/services/mod.rs", "pub mod workspace_mutation;")
remove_line("src/services/mod.rs", "pub mod workspace_read_model;")
remove_line("src/services/mod.rs", "pub mod projected_state;")
remove_line("src/services/mod.rs", "pub mod recovery;")
remove_line("src/services/mod.rs", "pub mod scanner;")

print("Imports updated for workspace module")
