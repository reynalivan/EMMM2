import os

modules_dir = "src/modules"
modules = [d for d in os.listdir(modules_dir) if os.path.isdir(os.path.join(modules_dir, d))]

for mod in modules:
    mod_rs = os.path.join(modules_dir, mod, "mod.rs")
    api_rs = os.path.join(modules_dir, mod, "api.rs")
    
    if os.path.exists(mod_rs) and os.path.exists(api_rs):
        with open(mod_rs, "r") as f:
            mod_content = f.read()
            
        has_adapters = "mod adapters;" in mod_content
        has_domain = "mod domain;" in mod_content
        has_application = "mod application;" in mod_content
        
        api_content = ""
        if has_application:
            api_content += "pub use super::application::*;\n"
            
        api_content += "\n#[cfg(debug_assertions)]\npub mod testing {\n"
        if has_adapters:
            api_content += "    pub mod adapters {\n        pub use super::super::super::adapters::*;\n    }\n"
        if has_domain:
            api_content += "    pub mod domain {\n        pub use super::super::super::domain::*;\n    }\n"
        if has_application:
            api_content += "    pub mod application {\n        pub use super::super::super::application::*;\n    }\n"
        api_content += "}\n"
        
        with open(api_rs, "w") as f:
            f.write(api_content)

print("Rewrote api.rs completely.")
