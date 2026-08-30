import os

replacements = {
    "emmm_lib::modules::system::application::config": "emmm_lib::modules::settings::application::config",
    "emmm_lib::modules::system::application::update": "emmm_lib::modules::updates::application::update",
    "emmm_lib::modules::workspace::application::scanner::deep_matcher": "emmm_lib::modules::matching::application::deep_matcher",
    "emmm_lib::modules::workspace::application::disk_reconcile": "emmm_lib::modules::reconciliation::application::disk_reconcile",
    "emmm_lib::app::runtime::mutation_coordinator": "emmm_lib::modules::mutation::coordinator",
    "emmm_lib::modules::workspace::application::workspace_mutation": "emmm_lib::modules::mutation::application::workspace_mutation",
    "emmm_lib::modules::storage_optimizer": "emmm_lib::modules::duplicates",
    "emmm_lib::modules::workspace::application::scanner::dedup": "emmm_lib::modules::duplicates::application::dedup",
}

def replace_in_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
            
        modified = False
        for old, new in replacements.items():
            if old in content:
                content = content.replace(old, new)
                modified = True
                
        if modified:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Updated: {filepath}")
    except Exception as e:
        print(f"Error reading {filepath}: {e}")

for root, _, files in os.walk("tests"):
    for file in files:
        if file.endswith(".rs"):
            replace_in_file(os.path.join(root, file))

print("Global path replacement for tests finished.")
