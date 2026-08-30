import os

mod_rs = "src-tauri/src/modules/mutation/mod.rs"
with open(mod_rs, "r") as f:
    content = f.read()

content = content.replace("pub mod journal;", "pub(crate) mod journal;")
content = content.replace("pub mod recovery;", "pub(crate) mod recovery;")
content = content.replace("pub mod task_registry;", "pub(crate) mod task_registry;")

with open(mod_rs, "w") as f:
    f.write(content)
print("Set mutation modules to pub(crate)")
