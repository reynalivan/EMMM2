import os

mapping = {
    # Ingestion
    "crate::services::import_batch": "crate::modules::ingestion::application::import_batch",
    "crate::repo::import_batch": "crate::modules::ingestion::adapters::outbound::sqlite::import_batch",
    "emmm_lib::services::import_batch": "emmm_lib::modules::ingestion::application::import_batch",
    "emmm_lib::repo::import_batch": "emmm_lib::modules::ingestion::adapters::outbound::sqlite::import_batch",
    
    # Catalog
    "crate::services::match_engine": "crate::modules::catalog::application::match_engine",
    "crate::services::objects": "crate::modules::catalog::application::objects",
    "crate::repo::object": "crate::modules::catalog::adapters::outbound::sqlite::object",
    "crate::domain::objects": "crate::modules::catalog::domain::objects",
    "emmm_lib::services::match_engine": "emmm_lib::modules::catalog::application::match_engine",
    "emmm_lib::services::objects": "emmm_lib::modules::catalog::application::objects",
    "emmm_lib::repo::object": "emmm_lib::modules::catalog::adapters::outbound::sqlite::object",
    "emmm_lib::domain::objects": "emmm_lib::modules::catalog::domain::objects",
    
    # Library
    "crate::services::mods": "crate::modules::library::application::mods",
    "crate::services::ini": "crate::modules::library::application::ini",
    "crate::services::apply_progress": "crate::modules::library::application::apply_progress",
    "crate::repo::mods": "crate::modules::library::adapters::outbound::sqlite::mods",
    "crate::domain::mods": "crate::modules::library::domain::mods",
    "emmm_lib::services::mods": "emmm_lib::modules::library::application::mods",
    "emmm_lib::services::ini": "emmm_lib::modules::library::application::ini",
    "emmm_lib::services::apply_progress": "emmm_lib::modules::library::application::apply_progress",
    "emmm_lib::repo::mods": "emmm_lib::modules::library::adapters::outbound::sqlite::mods",
    "emmm_lib::domain::mods": "emmm_lib::modules::library::domain::mods",
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

remove_line("src/repo/mod.rs", "pub mod import_batch;")
remove_line("src/repo/mod.rs", "pub mod object;")
remove_line("src/repo/mod.rs", "pub mod mods;")

remove_line("src/domain/mod.rs", "pub mod objects;")
remove_line("src/domain/mod.rs", "pub mod mods;")

remove_line("src/services/mod.rs", "pub mod import_batch;")
remove_line("src/services/mod.rs", "pub mod match_engine;")
remove_line("src/services/mod.rs", "pub mod objects;")
remove_line("src/services/mod.rs", "pub mod mods;")
remove_line("src/services/mod.rs", "pub mod ini;")
remove_line("src/services/mod.rs", "pub mod apply_progress;")

# add mods to lib.rs
with open("src/lib.rs", "r") as f:
    content = f.read()
if "pub mod ingestion;" not in content:
    content = content.replace("pub mod platform;", "pub mod platform;\n    pub mod ingestion;\n    pub mod catalog;\n    pub mod library;")
with open("src/lib.rs", "w") as f:
    f.write(content)

print("Imports updated for 3 large modules")
