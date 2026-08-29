import os

base = "src"
mapping = {
    "crate::domain::mod_path": "crate::modules::system::domain::mod_path",
    "crate::repo::settings": "crate::modules::system::adapters::outbound::sqlite::settings",
    "crate::repo::utils": "crate::modules::system::adapters::outbound::sqlite::utils",
    "crate::services::app": "crate::modules::system::application::app",
    "crate::services::config": "crate::modules::system::application::config",
    "crate::services::update": "crate::modules::system::application::update"
}

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

remove_line("src/services/mod.rs", "pub mod app;")
remove_line("src/services/mod.rs", "pub mod config;")
remove_line("src/services/mod.rs", "pub mod update;")
remove_line("src/repo/mod.rs", "pub mod settings;")
remove_line("src/repo/mod.rs", "pub mod utils;")
remove_line("src/domain/mod.rs", "pub mod mod_path;")

print("Imports fixed")
