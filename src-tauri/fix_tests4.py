import os, re
tests_dir = "src/modules/games/adapters/inbound/tests"
filepath = os.path.join(tests_dir, "game_cmds_tests.rs")
with open(filepath, "r") as f:
    content = f.read()

# Replace crate::commands::app::game_cmds:: with super::
content = content.replace("crate::commands::app::game_cmds::", "super::")

with open(filepath, "w") as f:
    f.write(content)

print("game_cmds_tests.rs fixed")
