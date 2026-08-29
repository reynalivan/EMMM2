import os
filepath = "src/modules/library/adapters/inbound/thumbnail_cmds.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("tests/mod_tests.rs", "tests/thumbnail_cmds_tests.rs")

with open(filepath, "w") as f:
    f.write(content)
