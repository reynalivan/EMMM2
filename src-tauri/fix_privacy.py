import os, re
filepath = "src/modules/mod.rs"
with open(filepath, "r") as f:
    content = f.read()
content = content.replace("pub mod privacy;\n", "")
with open(filepath, "w") as f:
    f.write(content)
print("Removed privacy mod")
