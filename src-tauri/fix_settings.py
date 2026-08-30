import os
import shutil

# Move the contents of application into application/config
os.makedirs("src/modules/settings/application/config", exist_ok=True)
files = ["models.rs", "persistence.rs", "service.rs", "tests"]
for f in files:
    src = f"src/modules/settings/application/{f}"
    if os.path.exists(src):
        shutil.move(src, "src/modules/settings/application/config/")

# Now fix mod.rs
with open("src/modules/settings/application/mod.rs", "w") as f:
    f.write("pub mod config;\n")

# And put back the original mod.rs for config
original_config_mod = """pub mod models;

mod persistence;
mod service;

pub use models::*;
pub use service::ConfigService;
"""
with open("src/modules/settings/application/config/mod.rs", "w") as f:
    f.write(original_config_mod)

print("Fixed settings/application/config structure.")
