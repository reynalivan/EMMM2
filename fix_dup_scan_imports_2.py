import os
import re

for root, _, files in os.walk("src-tauri"):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            try:
                with open(filepath, "r", encoding="utf-8") as f:
                    content = f.read()
                if "crate::types::dup_scan" in content:
                    content = content.replace("crate::types::dup_scan", "crate::modules::duplicates::api::domain")
                    with open(filepath, "w", encoding="utf-8") as f:
                        f.write(content)
                    print(f"Fixed {filepath}")
            except Exception as e:
                pass
