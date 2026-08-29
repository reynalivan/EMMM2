import os

# fix lib.rs
filepath = "src/lib.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::app::bootstrap", "crate::modules::system::application::app::bootstrap")
content = content.replace("services::config::ConfigService", "crate::modules::system::application::config::ConfigService")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)


# fix grouped import in persistence.rs
filepath = "src/modules/system/application/config/persistence.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("use crate::repo::{game, settings};", "use crate::repo::game;\nuse crate::modules::system::adapters::outbound::sqlite::settings;")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)


# fix bootstrap.rs
filepath = "src/modules/system/application/app/bootstrap.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()
content = content.replace("repo::utils::unicode_keys", "crate::modules::system::adapters::outbound::sqlite::utils::unicode_keys")
content = content.replace("services::config::ConfigService", "crate::modules::system::application::config::ConfigService")
with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed more system imports")
