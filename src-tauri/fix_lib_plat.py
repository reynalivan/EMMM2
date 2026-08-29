import os
filepath = "src/lib.rs"
with open(filepath, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("services::images::thumbnail_cache::ThumbnailCache", "crate::platform::images::thumbnail_cache::ThumbnailCache")
content = content.replace("services::fs_utils::operation_lock::OperationLock", "crate::platform::fs::operation_lock::OperationLock")

with open(filepath, "w", encoding="utf-8") as f:
    f.write(content)
print("Fixed lib.rs")
