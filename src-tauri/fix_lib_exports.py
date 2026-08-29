import os

filepath = "src/lib.rs"
with open(filepath, "r", encoding="utf-8") as f:
    lines = f.readlines()

new_lines = []
for l in lines:
    if l.strip() not in ["pub mod ingestion;", "pub mod catalog;", "pub mod library;"]:
        new_lines.append(l)

with open(filepath, "w", encoding="utf-8") as f:
    f.writelines(new_lines)

print("Fixed lib.rs root exports")
