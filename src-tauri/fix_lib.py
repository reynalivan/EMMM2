with open("src/lib.rs", "r") as f:
    content = f.read()

content = content.replace("modules::storage_optimizer", "crate::modules::duplicates")

with open("src/lib.rs", "w") as f:
    f.write(content)
print("Fixed lib.rs storage_optimizer imports.")
