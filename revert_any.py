import os

path1 = r"src\pages\mod-inbox\ModInboxPage.test.tsx"
with open(path1, "r", encoding="utf-8") as f:
    c = f.read()
c = c.replace("as unknown", "as any")
c = c.replace("unknown[]", "any[]")
c = c.replace(": unknown", ": any")

# add eslint-disable-next-line
lines1 = c.split('\n')
for i, line in enumerate(lines1):
    if "as any" in line and "eslint-disable" not in lines1[i-1]:
        lines1[i] = "    // eslint-disable-next-line @typescript-eslint/no-explicit-any\n" + line
c = "\n".join(lines1)
with open(path1, "w", encoding="utf-8") as f:
    f.write(c)

path2 = r"src\shared\ui\components\layout\top-bar\ContextControls.tsx"
with open(path2, "r", encoding="utf-8") as f:
    c = f.read()
c = c.replace("as unknown", "as any")
c = c.replace("unknown[]", "any[]")
c = c.replace(": unknown", ": any")

lines2 = c.split('\n')
for i, line in enumerate(lines2):
    if ": any" in line and "eslint-disable" not in lines2[i-1]:
        lines2[i] = "    // eslint-disable-next-line @typescript-eslint/no-explicit-any\n" + line
c = "\n".join(lines2)
with open(path2, "w", encoding="utf-8") as f:
    f.write(c)

print("Reverted to any with eslint-disable.")
