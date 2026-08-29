import os

base = "src"
mapping = {
    "crate::domain::collection": "crate::modules::collections::domain::collection",
    "crate::repo::collection": "crate::modules::collections::adapters::outbound::sqlite",
    "crate::services::collection_runtime": "crate::modules::collections::application::runtime",
    "crate::services::collection": "crate::modules::collections::application::collection",
    "crate::modules::collections::domain::collection_runtime": "crate::modules::collections::application::runtime" # catch mistakes if it matched part 1 and part 3 incorrectly
}

for root, dirs, files in os.walk(base):
    for f in files:
        if f.endswith(".rs"):
            filepath = os.path.join(root, f)
            with open(filepath, "r", encoding="utf-8") as file:
                content = file.read()
                
            original = content
            for old, new in mapping.items():
                content = content.replace(old, new)
                
            if content != original:
                with open(filepath, "w", encoding="utf-8") as file:
                    file.write(content)

print("Replaced imports")
