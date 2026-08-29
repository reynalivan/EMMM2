import os

base = "tests"
mapping = {
    "crate::domain::mod_path": "crate::modules::system::domain::mod_path",
    "crate::repo::settings": "crate::modules::system::adapters::outbound::sqlite::settings",
    "crate::repo::utils": "crate::modules::system::adapters::outbound::sqlite::utils",
    "crate::services::app": "crate::modules::system::application::app",
    "crate::services::config": "crate::modules::system::application::config",
    "crate::services::update": "crate::modules::system::application::update",
    
    "emmm_lib::domain::mod_path": "emmm_lib::modules::system::domain::mod_path",
    "emmm_lib::repo::settings": "emmm_lib::modules::system::adapters::outbound::sqlite::settings",
    "emmm_lib::repo::utils": "emmm_lib::modules::system::adapters::outbound::sqlite::utils",
    "emmm_lib::services::app": "emmm_lib::modules::system::application::app",
    "emmm_lib::services::config": "emmm_lib::modules::system::application::config",
    "emmm_lib::services::update": "emmm_lib::modules::system::application::update",
    
    "emmm_lib::domain::errors": "emmm_lib::shared::errors",
    "emmm_lib::services::fs_utils": "emmm_lib::platform::fs",
    "emmm_lib::services::images": "emmm_lib::platform::images",
    "emmm_lib::common::path_key": "emmm_lib::shared::path_key",
    "emmm_lib::common::sync": "emmm_lib::shared::sync"
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

print("Tests imports fixed")
