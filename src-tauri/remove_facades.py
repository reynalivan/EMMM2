import os
import re

for mod in ["browser", "collections", "duplicates"]:
    mod_rs = f"src/modules/{mod}/mod.rs"
    facade_rs = f"src/modules/{mod}/facade.rs"
    
    with open(mod_rs, "r") as f:
        content = f.read()
    
    content = re.sub(r'pub mod facade;\n?', '', content)
    content = re.sub(r'pub\(crate\) mod facade;\n?', '', content)
    
    with open(mod_rs, "w") as f:
        f.write(content)
        
    if os.path.exists(facade_rs):
        os.remove(facade_rs)

print("Removed all facade.rs files and references.")
