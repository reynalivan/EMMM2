import os
import re

modules_dir = "src/modules"
modules = [d for d in os.listdir(modules_dir) if os.path.isdir(os.path.join(modules_dir, d))]

for mod in modules:
    mod_rs = os.path.join(modules_dir, mod, "mod.rs")
    api_rs = os.path.join(modules_dir, mod, "api.rs")
    
    if os.path.exists(mod_rs) and os.path.exists(api_rs):
        with open(mod_rs, "r") as f:
            mod_content = f.read()
            
        with open(api_rs, "r") as f:
            api_content = f.read()
            
        api_content = re.sub(r'#\[cfg\(debug_assertions\)\].*?\}', '', api_content, flags=re.DOTALL)
        
        has_adapters = "mod adapters;" in mod_content
        has_domain = "mod domain;" in mod_content
        has_application = "mod application;" in mod_content
        
        testing_block = "\n#[cfg(debug_assertions)]\npub mod testing {\n"
        if has_adapters:
            testing_block += "    pub mod adapters {\n        pub use super::super::super::adapters::*;\n    }\n"
        if has_domain:
            testing_block += "    pub mod domain {\n        pub use super::super::super::domain::*;\n    }\n"
        if has_application:
            testing_block += "    pub mod application {\n        pub use super::super::super::application::*;\n    }\n"
        testing_block += "}\n"
        
        api_content = api_content.strip() + "\n" + testing_block
        
        with open(api_rs, "w") as f:
            f.write(api_content)

print("Fixed api.rs to only export existing modules.")
