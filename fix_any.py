import os

path1 = r"src\pages\mod-inbox\ModInboxPage.test.tsx"
with open(path1, "r", encoding="utf-8") as f:
    c = f.read()
c = c.replace("as any", "as unknown")
c = c.replace("any[]", "unknown[]")
c = c.replace(": any", ": unknown")
with open(path1, "w", encoding="utf-8") as f:
    f.write(c)

path2 = r"src\shared\ui\components\layout\top-bar\ContextControls.tsx"
with open(path2, "r", encoding="utf-8") as f:
    c = f.read()
c = c.replace("as any", "as unknown")
c = c.replace("any[]", "unknown[]")
c = c.replace(": any", ": unknown")
with open(path2, "w", encoding="utf-8") as f:
    f.write(c)

print("Fixed 'any' errors.")
