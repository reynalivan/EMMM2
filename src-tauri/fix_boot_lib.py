import os
filepath = "src/modules/system/application/app/bootstrap.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::import_batch", "crate::modules::ingestion::application::import_batch")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)

filepath = "src/lib.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::import_batch", "crate::modules::ingestion::application::import_batch")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed bootstrap and lib for 3 modules")
