import os, re
filepath = "src/lib.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("pub mod commands;", "")

with open(filepath, "w") as f:
    f.write(content)

print("Removed pub mod commands")
