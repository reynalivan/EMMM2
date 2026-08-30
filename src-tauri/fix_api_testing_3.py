import os
import re

modules_dir = "src/modules"
modules = [d for d in os.listdir(modules_dir) if os.path.isdir(os.path.join(modules_dir, d))]

for mod in modules:
    api_rs = os.path.join(modules_dir, mod, "api.rs")
    if os.path.exists(api_rs):
        with open(api_rs, "r") as f:
            content = f.read()
            
        # Remove the old testing block completely
        content = re.sub(r'#\[cfg\(debug_assertions\)\].*?\}', '', content, flags=re.DOTALL)
        
        # Add the new testing block
        new_testing = """
#[cfg(debug_assertions)]
pub mod testing {
    pub mod adapters {
        pub use super::super::super::adapters::*;
    }
    pub mod domain {
        pub use super::super::super::domain::*;
    }
    pub mod application {
        pub use super::super::super::application::*;
    }
}
"""
        content = content.strip() + "\n" + new_testing
        with open(api_rs, "w") as f:
            f.write(content)

print("Updated testing backdoor with nested modules.")
