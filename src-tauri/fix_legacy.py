import os
def remove_line(filepath, line):
    with open(filepath, "r") as f:
        content = f.read()
    content = content.replace(line + "\n", "")
    with open(filepath, "w") as f:
        f.write(content)

remove_line("src/domain/mod.rs", "pub mod collection;")
remove_line("src/repo/mod.rs", "pub mod collection;")
remove_line("src/services/mod.rs", "pub mod collection;")
remove_line("src/services/mod.rs", "pub mod collection_runtime;")

# fix duplicate outbound
filepath = "src/modules/collections/adapters/mod.rs"
with open(filepath, "r") as f:
    content = f.read()
content = content.replace("pub mod outbound;\npub mod outbound;\n", "pub mod outbound;\n")
with open(filepath, "w") as f:
    f.write(content)

# fix post_apply.rs which uses collection_runtime
filepath = "src/services/app/post_apply.rs"
with open(filepath, "r") as f:
    content = f.read()
content = content.replace("collection_runtime::", "crate::modules::collections::application::runtime::")
with open(filepath, "w") as f:
    f.write(content)

print("Fixed legacy mod declarations")
