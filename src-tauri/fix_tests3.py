import os
filepath = "tests/epic4_integration.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("emmm_lib::commands::mods::mod_core_cmds", "emmm_lib::modules::library::adapters::inbound::mod_core_cmds")

with open(filepath, "w") as f:
    f.write(content)
