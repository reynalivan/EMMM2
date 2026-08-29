import os

# Fix duplicate mods in browser
def fix_dups(filepath):
    with open(filepath, "r") as f:
        lines = f.readlines()
    seen = set()
    new_lines = []
    for l in lines:
        if l.strip() not in seen:
            seen.add(l.strip())
            new_lines.append(l)
    with open(filepath, "w") as f:
        f.writelines(new_lines)

fix_dups("src/modules/browser/adapters/mod.rs")
fix_dups("src/modules/browser/mod.rs")

# Fix hotkeys and browser in bootstrap.rs
filepath = "src/modules/system/application/app/bootstrap.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::hotkeys", "crate::modules::automation::application::hotkeys")
content = content.replace("repo::browser", "crate::modules::browser::adapters::outbound::sqlite::browser")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)

# Fix hotkeys in lib.rs
filepath = "src/lib.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::hotkeys", "crate::modules::automation::application::hotkeys")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed remaining 4 module errors")
