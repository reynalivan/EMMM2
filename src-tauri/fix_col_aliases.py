import os

base = "src"

for root, dirs, files in os.walk(base):
    for f in files:
        if f.endswith(".rs"):
            filepath = os.path.join(root, f)
            with open(filepath, "r", encoding="utf-8") as file:
                content = file.read()
                
            original = content
            content = content.replace("use crate::modules::collections::adapters::outbound::sqlite;", "use crate::modules::collections::adapters::outbound::sqlite as collection;")
            content = content.replace("use crate::modules::collections::adapters::outbound::sqlite::{", "use crate::modules::collections::adapters::outbound::sqlite::{") # wait, this doesn't help if they use `collection::get_by_id` and imported it via `use ...::sqlite;`. 
            # If it's `use ...::sqlite;`, `sqlite::get_by_id` would work. But they had `use crate::repo::collection;`
                
            if content != original:
                with open(filepath, "w", encoding="utf-8") as file:
                    file.write(content)

print("Replaced aliases")
