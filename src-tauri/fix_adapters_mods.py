import os

modules_dir = "src/modules"
for mod in os.listdir(modules_dir):
    mod_path = os.path.join(modules_dir, mod)
    if os.path.isdir(mod_path):
        adapters_dir = os.path.join(mod_path, "adapters")
        if os.path.isdir(adapters_dir):
            # check what dirs exist
            subdirs = [d for d in os.listdir(adapters_dir) if os.path.isdir(os.path.join(adapters_dir, d))]
            
            # create the mod.rs content
            lines = []
            for d in subdirs:
                lines.append(f"pub mod {d};\n")
                
            # If there's a file like 'something.rs', it also needs to be exported if it was.
            # But earlier it was just inbound/outbound.
            
            mod_file = os.path.join(adapters_dir, "mod.rs")
            with open(mod_file, "w") as f:
                f.writelines(lines)
                
            print(f"Fixed {mod_file}: {subdirs}")
