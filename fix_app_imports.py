import os

mod_rs = "src-tauri/src/modules/mutation/mod.rs"
with open(mod_rs, "r") as f:
    content = f.read()
if "pub mod journal;" not in content:
    content += "\npub mod journal;\npub mod recovery;\npub mod task_registry;\n"
    with open(mod_rs, "w") as f:
        f.write(content)

for root, _, files in os.walk("src-tauri/src"):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            try:
                with open(filepath, "r", encoding="utf-8") as f:
                    c = f.read()
                old_c = c
                c = c.replace("crate::app::runtime::operation_journal", "crate::modules::mutation::journal")
                c = c.replace("crate::app::runtime::recovery_runner", "crate::modules::mutation::recovery")
                c = c.replace("crate::app::runtime::task_registry", "crate::modules::mutation::task_registry")
                if c != old_c:
                    with open(filepath, "w", encoding="utf-8") as f:
                        f.write(c)
                    print(f"Fixed {filepath}")
            except Exception as e:
                pass
