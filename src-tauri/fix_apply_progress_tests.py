import os

base = "tests"

for root, dirs, files in os.walk(base):
    for f in files:
        if f.endswith(".rs"):
            filepath = os.path.join(root, f)
            with open(filepath, "r", encoding="utf-8") as file:
                content = file.read()
                
            original = content
            content = content.replace("emmm_lib::modules::system::application::apply_progress", "emmm_lib::services::apply_progress")
                
            if content != original:
                with open(filepath, "w", encoding="utf-8") as file:
                    file.write(content)

print("Fixed apply_progress in tests")
