import os
import re

modules_dir = "src/modules"
modules = [d for d in os.listdir(modules_dir) if os.path.isdir(os.path.join(modules_dir, d))]

for mod in modules:
    mod_path = os.path.join(modules_dir, mod)
    mod_rs = os.path.join(mod_path, "mod.rs")
    api_rs = os.path.join(mod_path, "api.rs")
    
    if os.path.exists(mod_rs):
        # 1. Modify mod.rs
        with open(mod_rs, "r") as f:
            content = f.read()
            
        # Replace pub mod domain/application/adapters with pub(crate) mod
        content = re.sub(r'pub mod domain;', 'pub(crate) mod domain;', content)
        content = re.sub(r'pub mod application;', 'pub(crate) mod application;', content)
        content = re.sub(r'pub mod adapters;', 'pub(crate) mod adapters;', content)
        
        # Add pub mod api; if not there
        if 'pub mod api;' not in content:
            content += "\npub mod api;\n"
            
        with open(mod_rs, "w") as f:
            f.write(content)
            
        # 2. Create api.rs
        if not os.path.exists(api_rs):
            api_content = ""
            if "pub(crate) mod application;" in content:
                api_content += "pub use super::application::*;\n"
            if "pub(crate) mod domain;" in content:
                api_content += "pub use super::domain::*;\n"
            
            # Write api.rs if it has something
            if api_content:
                with open(api_rs, "w") as f:
                    f.write(api_content)

print("Created api.rs and restricted visibility in mod.rs for all modules.")
