import os

path = r"src\shared\ui\components\layout\top-bar\ContextControls.tsx"
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if "eslint-disable-next-line @typescript-eslint/no-explicit-any" in line:
        lines[i] = ""
    elif "collections.map((c: any) =>" in line:
        lines[i] = line.replace("(c: any)", "(c)")

with open(path, "w", encoding="utf-8") as f:
    f.writelines(lines)
print("Fixed ContextControls.")
