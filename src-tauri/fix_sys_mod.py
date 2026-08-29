import os
filepath = "src/modules/system/mod.rs"
with open(filepath, "r") as f:
    content = f.read()
if "pub mod application;" not in content:
    with open(filepath, "a") as f:
        f.write("pub mod application;\npub mod domain;\n")
print("Fixed system/mod.rs")
