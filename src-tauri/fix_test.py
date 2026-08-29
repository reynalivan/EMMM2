import os
filepath = "src/modules/workspace/adapters/inbound/scanner_conflict_cmds.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("tests/conflict_cmds_tests.rs", "tests/scanner_conflict_cmds_tests.rs")

with open(filepath, "w") as f:
    f.write(content)

print("Updated path")
