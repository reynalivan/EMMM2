import os

mapping = {
    "crate::repo::dashboard": "crate::modules::dashboard::adapters::outbound::sqlite::dashboard",
    "crate::domain::dashboard": "crate::modules::dashboard::domain::dashboard",
    
    "crate::services::game": "crate::modules::games::application::game",
    "crate::repo::game": "crate::modules::games::adapters::outbound::sqlite::game",
    "crate::domain::models": "crate::modules::games::domain::models",
    
    "crate::services::hotkeys": "crate::modules::automation::application::hotkeys",
    "crate::services::keyviewer": "crate::modules::automation::application::keyviewer",
    
    "crate::services::browser": "crate::modules::browser::application::browser",
    "crate::repo::browser": "crate::modules::browser::adapters::outbound::sqlite::browser",
    "crate::domain::browser": "crate::modules::browser::domain::browser",
    
    "emmm_lib::repo::dashboard": "emmm_lib::modules::dashboard::adapters::outbound::sqlite::dashboard",
    "emmm_lib::domain::dashboard": "emmm_lib::modules::dashboard::domain::dashboard",
    
    "emmm_lib::services::game": "emmm_lib::modules::games::application::game",
    "emmm_lib::repo::game": "emmm_lib::modules::games::adapters::outbound::sqlite::game",
    "emmm_lib::domain::models": "emmm_lib::modules::games::domain::models",
    
    "emmm_lib::services::hotkeys": "emmm_lib::modules::automation::application::hotkeys",
    "emmm_lib::services::keyviewer": "emmm_lib::modules::automation::application::keyviewer",
    
    "emmm_lib::services::browser": "emmm_lib::modules::browser::application::browser",
    "emmm_lib::repo::browser": "emmm_lib::modules::browser::adapters::outbound::sqlite::browser",
    "emmm_lib::domain::browser": "emmm_lib::modules::browser::domain::browser"
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

remove_line("src/repo/mod.rs", "pub mod dashboard;")
remove_line("src/repo/mod.rs", "pub mod game;")
remove_line("src/repo/mod.rs", "pub mod browser;")

remove_line("src/domain/mod.rs", "pub mod dashboard;")
remove_line("src/domain/mod.rs", "pub mod models;")
remove_line("src/domain/mod.rs", "pub mod browser;")

remove_line("src/services/mod.rs", "pub mod game;")
remove_line("src/services/mod.rs", "pub mod hotkeys;")
remove_line("src/services/mod.rs", "pub mod keyviewer;")
remove_line("src/services/mod.rs", "pub mod browser;")

print("Imports updated for 4 modules")
