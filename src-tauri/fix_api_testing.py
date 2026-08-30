import os
import re

modules_dir = "src/modules"
modules = [d for d in os.listdir(modules_dir) if os.path.isdir(os.path.join(modules_dir, d))]

for mod in modules:
    api_rs = os.path.join(modules_dir, mod, "api.rs")
    if os.path.exists(api_rs):
        with open(api_rs, "r") as f:
            content = f.read()
            
        # Clean up previous glob re-exports to avoid ambiguity, 
        # actually if tests use `testing::application`, we don't need to re-export `application::*` globally for tests!
        # Wait, the app code itself uses api::* ? No, app code (like lib.rs) uses api::* too?
        # Let's remove the global globs and just put the testing module.
        # Wait, if I remove `pub use super::application::*;`, then lib.rs can't access it!
        # Let's keep `pub use super::application::*;` for now but fix the ambiguity by NOT exporting domain::*.
        
        content = re.sub(r'pub use super::domain::\*\;\n', '', content)
        
        if "pub mod testing" not in content:
            content += """
#[cfg(debug_assertions)]
pub mod testing {
    pub use super::super::adapters;
    pub use super::super::domain;
    pub use super::super::application;
}
"""
        with open(api_rs, "w") as f:
            f.write(content)

print("Added testing backdoor to api.rs")
