import os

replacements = {
    "crate::modules::system::application::config": "crate::modules::settings::application::config",
    "crate::modules::system::application::update": "crate::modules::updates::application::update",
    "crate::modules::workspace::application::scanner::deep_matcher": "crate::modules::matching::application::deep_matcher",
    "crate::modules::workspace::application::disk_reconcile": "crate::modules::reconciliation::application::disk_reconcile",
    "crate::modules::workspace::adapters::inbound::disk_reconcile_cmds": "crate::modules::reconciliation::adapters::inbound::disk_reconcile_cmds",
    "crate::app::runtime::mutation_coordinator": "crate::modules::mutation::coordinator",
    "crate::modules::workspace::application::workspace_mutation": "crate::modules::mutation::application::workspace_mutation",
    "crate::modules::storage_optimizer": "crate::modules::duplicates",
    "crate::modules::workspace::application::scanner::dedup": "crate::modules::duplicates::application::dedup",
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

for root, _, files in os.walk("src"):
    for file in files:
        if file.endswith(".rs"):
            replace_in_file(os.path.join(root, file))

print("Global path replacement finished.")
