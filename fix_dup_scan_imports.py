import os
import re

for root, _, files in os.walk("src-tauri"):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            with open(filepath, "r") as f:
                content = f.read()
            if "crate::types::dup_scan" in content:
                content = content.replace("crate::types::dup_scan", "crate::modules::duplicates::domain::dup_scan")
                with open(filepath, "w") as f:
                    f.write(content)
                print(f"Fixed {filepath}")
