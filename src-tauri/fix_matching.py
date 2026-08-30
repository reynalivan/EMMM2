path = "src/modules/matching/application/deep_matcher/mod.rs"
with open(path, "r") as f:
    content = f.read()

content = content.replace("use super::core::", "use crate::modules::workspace::application::scanner::core::")
content = content.replace("use super::sync::", "use crate::modules::workspace::application::scanner::sync::")

with open(path, "w") as f:
    f.write(content)
print("Fixed deep_matcher super::core imports.")
